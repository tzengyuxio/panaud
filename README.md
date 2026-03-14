# panaud

[![crates.io](https://img.shields.io/crates/v/panaud-cli.svg)](https://crates.io/crates/panaud-cli)
[![downloads](https://img.shields.io/crates/d/panaud-cli.svg)](https://crates.io/crates/panaud-cli)
[![docs.rs](https://docs.rs/panaud-core/badge.svg)](https://docs.rs/panaud-core)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE-MIT)
[![Rust](https://img.shields.io/badge/rust-2021_edition-orange.svg)](https://www.rust-lang.org/)

The Swiss Army knife of audio processing — built for humans and AI agents alike.

## Features

- **Multi-format support** — decode WAV, MP3, FLAC, OGG, AAC; encode WAV, MP3, FLAC (OGG opt-in)
- **Flexible time parsing** — `1:30`, `90s`, `1.5m`, `44100S` (samples)
- **AI-agent friendly** — structured JSON output, `--dry-run`, `--schema`, and `--capabilities` for programmatic use
- **Fast & safe** — built in Rust with structured error handling and exit codes

## Installation

### Homebrew (macOS / Linux)

```bash
brew install tzengyuxio/tap/panaud
```

### Cargo

```bash
cargo install panaud-cli
```

### Build from source

```bash
git clone https://github.com/tzengyuxio/panaud.git
cd panaud
cargo build --release
```

## Quick Start

```bash
# Get audio info
panaud info song.mp3 --format json

# Convert MP3 to WAV
panaud convert song.mp3 -o song.wav

# Trim audio to a time range
panaud trim song.wav -o clip.wav --start 1:30 --end 2:00

# Preview without executing
panaud trim song.wav -o clip.wav --start 1:30 --dry-run

# Adjust volume by +3 dB
panaud volume song.wav -o louder.wav --gain 3

# Peak-normalize audio
panaud normalize song.wav -o normalized.wav

# Fade in/out
panaud fade song.wav -o faded.wav --in 2s --out 3s

# Convert to mono
panaud channels song.wav -o mono.wav --mono

# Resample to 48 kHz
panaud resample song.wav -o resampled.wav --rate 48000

# Concatenate files
panaud concat intro.wav song.wav outro.wav -o full.wav

# Split into 4 equal parts
panaud split song.wav -o chunks/ --count 4
```

## Commands

| Command | Description |
|---------|-------------|
| `info` | Show audio file metadata (format, codec, sample rate, channels, duration) |
| `convert` | Convert audio between formats |
| `trim` | Trim audio to a time range |
| `volume` | Adjust audio volume |
| `normalize` | Peak-normalize audio |
| `fade` | Apply fade-in/fade-out to audio |
| `channels` | Change audio channel layout (mono, stereo, extract) |
| `resample` | Resample audio to a different sample rate |
| `concat` | Concatenate multiple audio files into one |
| `split` | Split audio into multiple files |

## Supported Formats

| Format | Decode | Encode |
|--------|--------|--------|
| WAV | ✅ symphonia | ✅ hound |
| MP3 | ✅ symphonia | ✅ mp3lame (default) |
| FLAC | ✅ symphonia | ✅ flacenc (default) |
| OGG | ✅ symphonia | ✅ vorbis_rs (opt-in) |
| AAC | ✅ symphonia | — |

> MP3 and FLAC encoding are enabled by default. OGG encoding requires the `ogg-enc` feature flag (`cargo install panaud-cli --features ogg-enc`) and a system libvorbis.

## Time Formats

The `--start` and `--end` flags accept flexible time formats:

| Format | Example | Meaning |
|--------|---------|---------|
| `mm:ss` | `1:30` | 1 minute 30 seconds |
| `hh:mm:ss` | `1:02:30` | 1 hour 2 min 30 sec |
| seconds | `90` | 90 seconds |
| `Ns` | `90s` | 90 seconds |
| `Nm` | `1.5m` | 1.5 minutes |
| `NS` | `44100S` | 44100 samples (capital S) |

## AI Agent Integration

panaud supports programmatic discovery and structured output for AI agents and automation:

```bash
panaud --capabilities --format json   # Discover all commands and formats
panaud info --schema                  # Get parameter definitions as JSON
panaud trim song.wav -o clip.wav --start 1:30 --dry-run --format json  # Preview without side effects
```

## Why panaud?

SoX (Sound eXchange) has been the "Swiss Army knife of sound processing" since 1991, but its last stable release (14.4.2) was in 2015. Development has effectively stopped, and its C codebase has known security vulnerabilities.

panaud aims to fill this gap with:

- **Explicit CLI syntax** — named flags instead of positional magic (`--channels mono` vs `remix -`)
- **AI-agent friendly** — `--format json`, `--dry-run`, `--schema`, `--capabilities`
- **Structured errors** — actionable error messages with suggestions
- **Modern tooling** — cross-platform binaries, batch processing, pipeline recipes

```bash
# SoX
sox input.wav output.flac remix - norm -3 highpass 22 gain -3 rate 48k

# panaud
panaud convert input.wav output.flac \
    --channels mono \
    --normalize -3 \
    --highpass 22 \
    --gain -3 \
    --sample-rate 48000
```

## Part of the pan- family

panaud is the second member of the pan- tool family, sharing core infrastructure (CLI framework, structured output, pipeline engine) via [`pan-common`](https://crates.io/crates/pan-common).

| Tool | Domain |
|------|--------|
| [panimg](https://github.com/tzengyuxio/panimg) | Image processing |
| **panaud** | Audio processing |

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT License ([LICENSE-MIT](LICENSE-MIT))

at your option.
