# panaud

> The Swiss Army knife of audio processing — built for humans and AI agents alike.

panaud is a modern replacement for [SoX](http://sox.sourceforge.net/) — built with Rust, designed with explicit CLI syntax and structured output for both human users and AI agents.

## Status

**v0.1.0** — MVP with `info`, `convert`, and `trim` commands.

| Feature | Decode | Encode |
|---------|--------|--------|
| WAV     | ✅     | ✅     |
| MP3     | ✅     | ❌     |
| FLAC    | ✅     | ❌     |
| OGG     | ✅     | ❌     |
| AAC     | ✅     | ❌     |

## Installation

```bash
cargo install --path crates/panaud-cli
```

## Usage

```bash
# Show audio file metadata
panaud info song.mp3
panaud info song.mp3 --format json

# Convert audio to WAV
panaud convert song.mp3 -o song.wav
panaud convert song.flac -o output.wav --overwrite

# Trim audio to a time range
panaud trim song.wav -o clip.wav --start 1:30 --end 2:00
panaud trim song.wav -o intro.wav --start 0 --end 30s

# Preview without executing
panaud trim song.wav -o clip.wav --start 1:30 --dry-run

# Show command schema (for AI agents)
panaud info --schema
panaud convert --schema

# List all capabilities
panaud --capabilities
panaud --capabilities --format json
```

### Time formats

The `--start` and `--end` flags accept flexible time formats:

| Format    | Example   | Meaning          |
|-----------|-----------|------------------|
| `mm:ss`   | `1:30`    | 1 minute 30 seconds |
| `hh:mm:ss`| `1:02:30` | 1 hour 2 min 30 sec |
| seconds   | `90`      | 90 seconds       |
| `Ns`      | `90s`     | 90 seconds       |
| `Nm`      | `1.5m`    | 1.5 minutes      |
| `NS`      | `44100S`  | 44100 samples    |

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

## Architecture

```
panaud/
├── crates/
│   ├── panaud-core/     ← library: codec, ops, types, info
│   └── panaud-cli/      ← binary: CLI interface (clap)
└── Cargo.toml           ← workspace root
```

- **pan-common** — shared infrastructure (Pipeline, Operation trait, structured errors, output formatting)
- **panaud-core** — audio-specific logic: symphonia decoding, hound WAV encoding, TrimOp
- **panaud-cli** — command dispatch, argument parsing, human/JSON output

## Part of the pan- family

panaud is the second member of the pan- tool family, sharing core infrastructure (CLI framework, structured output, batch processing) via `pan-common`.

| Tool | Domain |
|------|--------|
| [panimg](https://github.com/tzengyuxio/panimg) | Image processing |
| **panaud** | Audio processing |

## License

MIT
