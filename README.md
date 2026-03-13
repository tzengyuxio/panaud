# panaud

> The Swiss Army knife of audio processing — built for humans and AI agents alike.

panaud is a modern replacement for [SoX](http://sox.sourceforge.net/) — built with Rust, designed with explicit CLI syntax and structured output for both human users and AI agents.

## Status

🚧 **Planning phase** — not yet functional.

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

panaud is the second member of the pan- tool family, sharing core infrastructure (CLI framework, structured output, batch processing) via `pan-common`.

## License

MIT
