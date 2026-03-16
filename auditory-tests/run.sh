#!/usr/bin/env bash
#
# Auditory test suite for panaud.
# Generates test audio, runs all panaud operations, and produces an HTML gallery
# for human listening evaluation (A/B comparison).
#
# Usage:
#   bash auditory-tests/run.sh
#
# Requirements:
#   - Rust toolchain (cargo)
#   - ffmpeg (for generating test audio sources)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
RESULTS_DIR="$SCRIPT_DIR/results"
TMP_DIR="$SCRIPT_DIR/tmp"
PANAUD="$PROJECT_ROOT/target/release/panaud"

PASS=0
FAIL=0
TOTAL=0

# Test results stored as parallel arrays
declare -a TEST_NUMS=()
declare -a TEST_DESCS=()
declare -a TEST_CMDS=()
declare -a TEST_STATUSES=()
declare -a TEST_SRC_FILES=()
declare -a TEST_OUT_FILES=()   # semicolon-separated for split tests
declare -a TEST_GALLERY=()     # "yes" or "no" — whether to show in gallery

# Metadata cache directory (file-based, compatible with bash 3)
META_CACHE_DIR=""

# ── Helpers ──────────────────────────────────────────────────────────────────

info() { printf "\033[1;34m[INFO]\033[0m %s\n" "$*"; }
pass() { printf "\033[1;32m[PASS]\033[0m %s\n" "$*"; }
fail() { printf "\033[1;31m[FAIL]\033[0m %s\n" "$*"; }

get_meta() {
    local file="$1"
    if [[ ! -f "$file" ]]; then
        echo "N/A|N/A|N/A"
        return
    fi
    # File-based cache using md5 hash of path as key
    local cache_key cache_file
    cache_key=$(echo -n "$file" | md5 -q 2>/dev/null || echo -n "$file" | md5sum | cut -d' ' -f1)
    cache_file="$META_CACHE_DIR/$cache_key"
    if [[ -n "$META_CACHE_DIR" && -f "$cache_file" ]]; then
        cat "$cache_file"
        return
    fi
    local json
    json=$("$PANAUD" info "$file" --format json 2>/dev/null || echo "{}")
    local dur sr ch
    dur=$(echo "$json" | jq -r '.duration_secs // "N/A"' 2>/dev/null || echo "N/A")
    sr=$(echo "$json" | jq -r '.sample_rate // "N/A"' 2>/dev/null || echo "N/A")
    ch=$(echo "$json" | jq -r '.channels // "N/A"' 2>/dev/null || echo "N/A")
    if [[ "$dur" != "N/A" ]]; then
        dur=$(printf "%.2f" "$dur")
    fi
    local result="${dur}s|${sr}|${ch}"
    if [[ -n "$META_CACHE_DIR" ]]; then
        echo "$result" > "$cache_file"
    fi
    echo "$result"
}

# Core test execution: sets _LAST_STATUS to "PASS" or "FAIL"
_exec_test() {
    local num="$1"
    local description="$2"
    shift 2

    TOTAL=$((TOTAL + 1))

    info "[$num] $description"
    info "  \$ $*"

    local exit_code=0
    eval "$@" 2>&1 || exit_code=$?

    if [[ $exit_code -ne 0 ]]; then
        fail "[$num] $description (exit code: $exit_code)"
        FAIL=$((FAIL + 1))
        _LAST_STATUS="FAIL"
    else
        pass "[$num] $description"
        PASS=$((PASS + 1))
        _LAST_STATUS="PASS"
    fi
}

run_test() {
    local num="$1"
    local description="$2"
    local src_file="$3"
    local out_file="$4"
    local gallery="${5:-yes}"
    shift 5

    _exec_test "$num" "$description" "$@"

    TEST_NUMS+=("$num")
    TEST_DESCS+=("$description")
    TEST_CMDS+=("$*")
    TEST_STATUSES+=("$_LAST_STATUS")
    TEST_SRC_FILES+=("$src_file")
    TEST_OUT_FILES+=("$out_file")
    TEST_GALLERY+=("$gallery")
}

run_split_test() {
    local num="$1"
    local description="$2"
    local src_file="$3"
    local out_dir="$4"
    local expected_count="$5"
    shift 5

    _exec_test "$num" "$description" "$@"

    # Collect split output files and copy to results root for gallery playback
    local out_files_str=""
    for i in $(seq 1 "$expected_count"); do
        local f="$out_dir/source_$(printf "%03d" "$i").wav"
        [[ -n "$out_files_str" ]] && out_files_str="${out_files_str};"
        out_files_str="${out_files_str}${f}"
        if [[ -f "$f" ]]; then
            cp "$f" "$RESULTS_DIR/${num}_split_part${i}.wav"
        fi
    done

    TEST_NUMS+=("$num")
    TEST_DESCS+=("$description")
    TEST_CMDS+=("$*")
    TEST_STATUSES+=("$_LAST_STATUS")
    TEST_SRC_FILES+=("$src_file")
    TEST_OUT_FILES+=("$out_files_str")
    TEST_GALLERY+=("yes")
}

# ── Pre-flight checks ───────────────────────────────────────────────────────

info "Checking prerequisites..."

if ! command -v ffmpeg &>/dev/null; then
    fail "ffmpeg not found. Install it with: brew install ffmpeg"
    exit 1
fi

if ! command -v jq &>/dev/null; then
    fail "jq not found. Install it with: brew install jq"
    exit 1
fi

info "Building panaud in release mode..."
(cd "$PROJECT_ROOT" && cargo build --release -p panaud-cli)

if [[ ! -x "$PANAUD" ]]; then
    fail "panaud binary not found at $PANAUD"
    exit 1
fi

info "panaud version: $("$PANAUD" --version)"

# ── Setup directories ────────────────────────────────────────────────────────

rm -rf "$RESULTS_DIR" "$TMP_DIR"
mkdir -p "$RESULTS_DIR" "$TMP_DIR"
META_CACHE_DIR="$TMP_DIR/.meta_cache"
mkdir -p "$META_CACHE_DIR"

# ── Generate test audio sources ──────────────────────────────────────────────

info "Generating test audio source..."

# Create a ~5s stereo WAV (44100 Hz) with distinct L/R content.
# Left: 440 Hz sine, Right: 445 Hz sine (slight detuning for stereo distinction)
ffmpeg -y -loglevel error \
    -f lavfi -i "sine=frequency=440:duration=5:sample_rate=44100" \
    -f lavfi -i "sine=frequency=445:duration=5:sample_rate=44100" \
    -filter_complex "[0][1]amerge=inputs=2[stereo]" \
    -map "[stereo]" \
    -ar 44100 \
    "$TMP_DIR/source.wav" 2>&1 || true

# Fallback: pink noise
if [[ ! -f "$TMP_DIR/source.wav" ]]; then
    info "Stereo sine failed, falling back to pink noise..."
    ffmpeg -y -loglevel error \
        -f lavfi -i "anoisesrc=d=5:c=pink:r=44100:a=0.5" \
        -ac 2 \
        "$TMP_DIR/source.wav"
fi

SOURCE="$TMP_DIR/source.wav"

# Copy source to results for gallery reference
cp "$SOURCE" "$RESULTS_DIR/source.wav"

info "Source audio generated: $(get_meta "$SOURCE")"

# Create source_b.wav (second half, for concat test)
"$PANAUD" trim "$SOURCE" -o "$TMP_DIR/source_b.wav" --start 2.5s --overwrite
info "Source B (for concat) generated: $(get_meta "$TMP_DIR/source_b.wav")"

# Create a mono version for stereo conversion test
"$PANAUD" channels "$SOURCE" -o "$TMP_DIR/source_mono.wav" --mono --overwrite
cp "$TMP_DIR/source_mono.wav" "$RESULTS_DIR/source_mono.wav"
info "Mono source generated: $(get_meta "$TMP_DIR/source_mono.wav")"

echo ""
info "═══════════════════════════════════════════════════════════════"
info "  Running auditory tests"
info "═══════════════════════════════════════════════════════════════"
echo ""

# ── Test cases ───────────────────────────────────────────────────────────────

# --- Volume & Normalize ---

run_test "01" "Volume: +6 dB gain" \
    "$SOURCE" "$RESULTS_DIR/01_volume_up.wav" yes \
    "$PANAUD" volume "$SOURCE" -o "$RESULTS_DIR/01_volume_up.wav" --gain 6 --overwrite

run_test "02" "Volume: -12 dB gain" \
    "$SOURCE" "$RESULTS_DIR/02_volume_down.wav" yes \
    "$PANAUD" volume "$SOURCE" -o "$RESULTS_DIR/02_volume_down.wav" --gain=-12 --overwrite

run_test "03" "Volume: factor 0.25" \
    "$SOURCE" "$RESULTS_DIR/03_volume_factor.wav" yes \
    "$PANAUD" volume "$SOURCE" -o "$RESULTS_DIR/03_volume_factor.wav" --factor 0.25 --overwrite

run_test "04" "Normalize to -1 dBFS" \
    "$SOURCE" "$RESULTS_DIR/04_normalize.wav" yes \
    "$PANAUD" normalize "$SOURCE" -o "$RESULTS_DIR/04_normalize.wav" --target=-1 --overwrite

# --- Fade ---

run_test "05" "Fade in: 2s" \
    "$SOURCE" "$RESULTS_DIR/05_fade_in.wav" yes \
    "$PANAUD" fade "$SOURCE" -o "$RESULTS_DIR/05_fade_in.wav" --in 2s --overwrite

run_test "06" "Fade out: 2s" \
    "$SOURCE" "$RESULTS_DIR/06_fade_out.wav" yes \
    "$PANAUD" fade "$SOURCE" -o "$RESULTS_DIR/06_fade_out.wav" --out 2s --overwrite

run_test "07" "Fade in 1s + out 1s" \
    "$SOURCE" "$RESULTS_DIR/07_fade_both.wav" yes \
    "$PANAUD" fade "$SOURCE" -o "$RESULTS_DIR/07_fade_both.wav" --in 1s --out 1s --overwrite

# --- Trim ---

run_test "08" "Trim: 1s to 3s" \
    "$SOURCE" "$RESULTS_DIR/08_trim_middle.wav" yes \
    "$PANAUD" trim "$SOURCE" -o "$RESULTS_DIR/08_trim_middle.wav" --start 1s --end 3s --overwrite

run_test "09" "Trim: 0s to 2s" \
    "$SOURCE" "$RESULTS_DIR/09_trim_start.wav" yes \
    "$PANAUD" trim "$SOURCE" -o "$RESULTS_DIR/09_trim_start.wav" --start 0s --end 2s --overwrite

# --- Channels ---

run_test "10" "Channels: stereo to mono" \
    "$SOURCE" "$RESULTS_DIR/10_channels_mono.wav" yes \
    "$PANAUD" channels "$SOURCE" -o "$RESULTS_DIR/10_channels_mono.wav" --mono --overwrite

run_test "11" "Channels: mono to stereo" \
    "$TMP_DIR/source_mono.wav" "$RESULTS_DIR/11_channels_stereo.wav" yes \
    "$PANAUD" channels "$TMP_DIR/source_mono.wav" -o "$RESULTS_DIR/11_channels_stereo.wav" --stereo --overwrite

run_test "12" "Channels: extract left" \
    "$SOURCE" "$RESULTS_DIR/12_extract_left.wav" yes \
    "$PANAUD" channels "$SOURCE" -o "$RESULTS_DIR/12_extract_left.wav" --extract left --overwrite

run_test "13" "Channels: extract right" \
    "$SOURCE" "$RESULTS_DIR/13_extract_right.wav" yes \
    "$PANAUD" channels "$SOURCE" -o "$RESULTS_DIR/13_extract_right.wav" --extract right --overwrite

# --- Resample ---

run_test "14" "Resample: 44100 → 22050 Hz" \
    "$SOURCE" "$RESULTS_DIR/14_resample_down.wav" yes \
    "$PANAUD" resample "$SOURCE" -o "$RESULTS_DIR/14_resample_down.wav" --rate 22050 --overwrite

run_test "15" "Resample: 44100 → 96000 Hz" \
    "$SOURCE" "$RESULTS_DIR/15_resample_up.wav" yes \
    "$PANAUD" resample "$SOURCE" -o "$RESULTS_DIR/15_resample_up.wav" --rate 96000 --overwrite

run_test "16" "Resample: 44100 → 48000 Hz" \
    "$SOURCE" "$RESULTS_DIR/16_resample_48k.wav" yes \
    "$PANAUD" resample "$SOURCE" -o "$RESULTS_DIR/16_resample_48k.wav" --rate 48000 --overwrite

# --- Convert ---

run_test "17" "Convert: WAV → MP3" \
    "$SOURCE" "$RESULTS_DIR/17_convert_mp3.mp3" yes \
    "$PANAUD" convert "$SOURCE" -o "$RESULTS_DIR/17_convert_mp3.mp3" --overwrite

run_test "18" "Convert: WAV → FLAC" \
    "$SOURCE" "$RESULTS_DIR/18_convert_flac.flac" yes \
    "$PANAUD" convert "$SOURCE" -o "$RESULTS_DIR/18_convert_flac.flac" --overwrite

# --- Concat ---

run_test "19" "Concat: source + source_b" \
    "$SOURCE" "$RESULTS_DIR/19_concat.wav" yes \
    "$PANAUD" concat "$SOURCE" "$TMP_DIR/source_b.wav" -o "$RESULTS_DIR/19_concat.wav" --overwrite

# --- Split ---

run_split_test "20" "Split: into 3 equal parts" \
    "$SOURCE" "$RESULTS_DIR/20_split" 3 \
    "$PANAUD" split "$SOURCE" -o "$RESULTS_DIR/20_split" --count 3 --overwrite

# --- Pipeline (multi-step) ---

# Pipeline 1: trim → volume → fade
info "[21] Pipeline: trim → volume → fade"
run_test "21a" "Pipeline step 1: trim 0.5s–4.5s" \
    "$SOURCE" "$TMP_DIR/21_step1_trim.wav" no \
    "$PANAUD" trim "$SOURCE" -o "$TMP_DIR/21_step1_trim.wav" --start 0.5s --end 4.5s --overwrite

run_test "21b" "Pipeline step 2: volume +3 dB" \
    "$TMP_DIR/21_step1_trim.wav" "$TMP_DIR/21_step2_vol.wav" no \
    "$PANAUD" volume "$TMP_DIR/21_step1_trim.wav" -o "$TMP_DIR/21_step2_vol.wav" --gain 3 --overwrite

run_test "21" "Pipeline: trim → volume → fade" \
    "$SOURCE" "$RESULTS_DIR/21_pipeline_fade_volume.wav" yes \
    "$PANAUD" fade "$TMP_DIR/21_step2_vol.wav" -o "$RESULTS_DIR/21_pipeline_fade_volume.wav" --in 0.5s --out 0.5s --overwrite

# Pipeline 2: resample → mono
info "[22] Pipeline: resample → channels"
run_test "22a" "Pipeline step 1: resample to 22050" \
    "$SOURCE" "$TMP_DIR/22_step1_resample.wav" no \
    "$PANAUD" resample "$SOURCE" -o "$TMP_DIR/22_step1_resample.wav" --rate 22050 --overwrite

run_test "22" "Pipeline: resample → mono" \
    "$SOURCE" "$RESULTS_DIR/22_pipeline_resample_mono.wav" yes \
    "$PANAUD" channels "$TMP_DIR/22_step1_resample.wav" -o "$RESULTS_DIR/22_pipeline_resample_mono.wav" --mono --overwrite

echo ""
info "═══════════════════════════════════════════════════════════════"
info "  Generating HTML gallery"
info "═══════════════════════════════════════════════════════════════"
echo ""

# ── HTML Gallery Generation ──────────────────────────────────────────────────

GALLERY="$RESULTS_DIR/gallery.html"

cat > "$GALLERY" <<'HTMLHEAD'
<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>panaud — Auditory Test Gallery</title>
<style>
  :root {
    --bg: #1a1a2e;
    --card-bg: #16213e;
    --card-border: #0f3460;
    --text: #e0e0e0;
    --text-dim: #888;
    --accent: #e94560;
    --pass: #4ade80;
    --fail: #f87171;
    --code-bg: #0d1117;
  }
  * { box-sizing: border-box; margin: 0; padding: 0; }
  body {
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, monospace;
    background: var(--bg);
    color: var(--text);
    padding: 2rem;
    max-width: 1000px;
    margin: 0 auto;
  }
  h1 {
    font-size: 1.8rem;
    margin-bottom: 0.5rem;
    color: var(--accent);
  }
  .summary {
    margin-bottom: 2rem;
    padding: 1rem;
    background: var(--card-bg);
    border: 1px solid var(--card-border);
    border-radius: 8px;
    font-size: 1rem;
  }
  .summary .pass-count { color: var(--pass); font-weight: bold; }
  .summary .fail-count { color: var(--fail); font-weight: bold; }
  .card {
    background: var(--card-bg);
    border: 1px solid var(--card-border);
    border-radius: 8px;
    margin-bottom: 1.5rem;
    overflow: hidden;
  }
  .card-header {
    padding: 0.8rem 1rem;
    font-weight: bold;
    font-size: 1.1rem;
    border-bottom: 1px solid var(--card-border);
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  .badge {
    font-size: 0.75rem;
    padding: 2px 8px;
    border-radius: 4px;
    font-weight: bold;
  }
  .badge.pass { background: var(--pass); color: #000; }
  .badge.fail { background: var(--fail); color: #000; }
  .players {
    display: flex;
    gap: 1rem;
    padding: 1rem;
    flex-wrap: wrap;
  }
  .player {
    flex: 1;
    min-width: 200px;
  }
  .player .label {
    font-size: 0.85rem;
    color: var(--text-dim);
    margin-bottom: 0.4rem;
    font-weight: 600;
  }
  audio {
    width: 100%;
    height: 40px;
    border-radius: 4px;
  }
  .meta {
    padding: 0.8rem 1rem;
    border-top: 1px solid var(--card-border);
    font-size: 0.85rem;
  }
  .meta code {
    display: block;
    background: var(--code-bg);
    padding: 0.5rem 0.8rem;
    border-radius: 4px;
    margin-bottom: 0.6rem;
    overflow-x: auto;
    white-space: nowrap;
    color: #9ca3af;
  }
  .meta table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.8rem;
  }
  .meta th, .meta td {
    padding: 3px 8px;
    text-align: left;
    border-bottom: 1px solid var(--card-border);
  }
  .meta th { color: var(--text-dim); }
  .listen-note {
    font-size: 0.8rem;
    color: var(--text-dim);
    margin-bottom: 1.5rem;
  }
</style>
</head>
<body>
<h1>panaud — Auditory Test Gallery</h1>
<p class="listen-note">Compare Source vs Result for each test. Use headphones for best results.</p>
HTMLHEAD

# Count gallery tests for summary
gallery_count=0
gallery_pass=0
gallery_fail=0
for i in "${!TEST_GALLERY[@]}"; do
    [[ "${TEST_GALLERY[$i]}" != "yes" ]] && continue
    gallery_count=$((gallery_count + 1))
    [[ "${TEST_STATUSES[$i]}" == "PASS" ]] && gallery_pass=$((gallery_pass + 1))
    [[ "${TEST_STATUSES[$i]}" == "FAIL" ]] && gallery_fail=$((gallery_fail + 1))
done

{
    echo "<div class=\"summary\">"
    echo "  Tests: <strong>$gallery_count</strong> |"
    echo "  <span class=\"pass-count\">Pass: $gallery_pass</span> |"
    echo "  <span class=\"fail-count\">Fail: $gallery_fail</span>"
    echo "</div>"
} >> "$GALLERY"

# Helper: resolve source file to a gallery-relative path
src_rel_path() {
    local src_file="$1"
    if [[ "$src_file" == *"/source_mono.wav" ]]; then
        echo "source_mono.wav"
    else
        echo "source.wav"
    fi
}

# Write test cards
for i in "${!TEST_NUMS[@]}"; do
    [[ "${TEST_GALLERY[$i]}" != "yes" ]] && continue

    num="${TEST_NUMS[$i]}"
    desc="${TEST_DESCS[$i]}"
    cmd="${TEST_CMDS[$i]}"
    status="${TEST_STATUSES[$i]}"
    src_file="${TEST_SRC_FILES[$i]}"
    out_file="${TEST_OUT_FILES[$i]}"

    badge_class="pass"
    [[ "$status" == "FAIL" ]] && badge_class="fail"

    # Replace absolute paths with relative filenames for display
    cmd_display="${cmd//$RESULTS_DIR\//}"
    cmd_display="${cmd_display//$TMP_DIR\//tmp/}"
    cmd_display="${cmd_display//$PROJECT_ROOT\/target\/release\//}"

    src_rel=$(src_rel_path "$src_file")
    IFS='|' read -r src_dur src_sr src_ch <<< "$(get_meta "$src_file")"

    # Card header (shared by both normal and split)
    {
        echo "<div class=\"card\">"
        echo "  <div class=\"card-header\">"
        echo "    <span>[$num] $desc</span>"
        echo "    <span class=\"badge $badge_class\">$status</span>"
        echo "  </div>"
        echo "  <div class=\"players\">"
        echo "    <div class=\"player\">"
        echo "      <div class=\"label\">Source</div>"
        echo "      <audio controls src=\"$src_rel\" preload=\"none\"></audio>"
        echo "    </div>"
    } >> "$GALLERY"

    if [[ "$out_file" == *";"* ]]; then
        # Split test — multiple output files
        IFS=';' read -ra split_files <<< "$out_file"
        {
            for p in $(seq 1 ${#split_files[@]}); do
                echo "    <div class=\"player\">"
                echo "      <div class=\"label\">Part $p</div>"
                echo "      <audio controls src=\"${num}_split_part${p}.wav\" preload=\"none\"></audio>"
                echo "    </div>"
            done
            echo "  </div>"
            echo "  <div class=\"meta\">"
            echo "    <code>\$ $cmd_display</code>"
            echo "    <table>"
            echo "      <tr><th></th><th>Source</th>"
            for p in $(seq 1 ${#split_files[@]}); do
                echo "        <th>Part $p</th>"
            done
            echo "      </tr>"
            echo "      <tr><td>Duration</td><td>$src_dur</td>"
            for sf in "${split_files[@]}"; do
                IFS='|' read -r d _ _ <<< "$(get_meta "$sf")"
                echo "        <td>$d</td>"
            done
            echo "      </tr>"
            echo "      <tr><td>Sample Rate</td><td>$src_sr</td>"
            for sf in "${split_files[@]}"; do
                IFS='|' read -r _ s _ <<< "$(get_meta "$sf")"
                echo "        <td>$s</td>"
            done
            echo "      </tr>"
            echo "      <tr><td>Channels</td><td>$src_ch</td>"
            for sf in "${split_files[@]}"; do
                IFS='|' read -r _ _ c <<< "$(get_meta "$sf")"
                echo "        <td>$c</td>"
            done
            echo "      </tr>"
            echo "    </table>"
            echo "  </div>"
        } >> "$GALLERY"
    else
        # Normal test — single output file
        IFS='|' read -r out_dur out_sr out_ch <<< "$(get_meta "$out_file")"
        out_base=$(basename "$out_file" 2>/dev/null || echo "")
        {
            echo "    <div class=\"player\">"
            echo "      <div class=\"label\">Result</div>"
            echo "      <audio controls src=\"$out_base\" preload=\"none\"></audio>"
            echo "    </div>"
            echo "  </div>"
            echo "  <div class=\"meta\">"
            echo "    <code>\$ $cmd_display</code>"
            echo "    <table>"
            echo "      <tr><th></th><th>Source</th><th>Result</th></tr>"
            echo "      <tr><td>Duration</td><td>$src_dur</td><td>$out_dur</td></tr>"
            echo "      <tr><td>Sample Rate</td><td>$src_sr</td><td>$out_sr</td></tr>"
            echo "      <tr><td>Channels</td><td>$src_ch</td><td>$out_ch</td></tr>"
            echo "    </table>"
            echo "  </div>"
        } >> "$GALLERY"
    fi

    echo "</div>" >> "$GALLERY"
done

cat >> "$GALLERY" <<'HTMLFOOT'
<p style="margin-top: 2rem; color: #555; font-size: 0.75rem; text-align: center;">
  Generated by panaud auditory-tests
</p>
</body>
</html>
HTMLFOOT

info "Gallery written to: $GALLERY"

# ── Cleanup ──────────────────────────────────────────────────────────────────

rm -rf "$TMP_DIR"

echo ""
info "═══════════════════════════════════════════════════════════════"
info "  Results"
info "═══════════════════════════════════════════════════════════════"
echo ""
info "Total: $TOTAL | Pass: $PASS | Fail: $FAIL"
echo ""
info "Open the gallery:"
info "  open $RESULTS_DIR/gallery.html"
echo ""
