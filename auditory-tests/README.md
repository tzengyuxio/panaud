# Auditory Tests

Human-in-the-loop test suite for panaud. Runs all audio processing operations
and generates an HTML gallery with side-by-side A/B players for listening
evaluation.

## Prerequisites

- Rust toolchain (`cargo`)
- `ffmpeg` (for generating test audio)
- `jq` (for parsing metadata)

## Usage

```bash
bash auditory-tests/run.sh
open auditory-tests/results/gallery.html
```

The script will:

1. Build `panaud-cli` in release mode
2. Generate a 5-second stereo test source via `ffmpeg`
3. Run 22 main tests (+ 3 pipeline intermediate steps)
4. Produce an HTML gallery with Source vs Result audio players

## What it tests

| Category | Tests | What to listen for |
|----------|-------|--------------------|
| Volume | +6dB, -12dB, factor 0.25, normalize | Loudness change, no clipping |
| Fade | In, out, both | Smooth transitions, no clicks |
| Trim | Middle section, start | Correct segment extracted |
| Channels | Mono, stereo, extract L/R | Correct spatial behavior |
| Resample | 22050, 48000, 96000 Hz | Quality preservation |
| Convert | MP3, FLAC | Lossy vs lossless fidelity |
| Concat | Two files joined | Seamless join, no gap/click |
| Split | 3 equal parts | Clean cuts, no artifacts |
| Pipeline | trim→volume→fade, resample→mono | Combined effects correct |

## Output

Results are written to `auditory-tests/results/` (gitignored):

- `gallery.html` — Dark-themed HTML page with `<audio>` players
- `source.wav` — Reference stereo audio for comparison
- `source_mono.wav` — Reference mono audio (for mono→stereo test)
- `NN_*.wav/mp3/flac` — Individual test outputs
