use criterion::{criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion, Throughput};
use panaud_core::ops::channels::{ChannelMode, ChannelSelector, ChannelsOp};
use panaud_core::ops::concat::concat_audio;
use panaud_core::ops::fade::FadeOp;
use panaud_core::ops::normalize::NormalizeOp;
use panaud_core::ops::split::{split_audio, SplitMode};
use panaud_core::ops::trim::TrimOp;
use panaud_core::ops::volume::VolumeOp;
use panaud_core::ops::Operation;
use panaud_core::time::TimeSpec;
use panaud_core::types::{AudioData, AudioFormat};

#[cfg(feature = "resample")]
use panaud_core::ops::resample::ResampleOp;

use panaud_core::codec::CodecRegistry;

const DURATIONS: &[(f64, &str)] = &[(1.0, "1s"), (10.0, "10s"), (60.0, "60s")];

/// Generate a 440Hz sine wave at the given sample rate.
fn generate_sine(duration_secs: f64, sample_rate: u32, channels: u16) -> AudioData {
    let num_frames = (duration_secs * sample_rate as f64) as usize;
    let total_samples = num_frames * channels as usize;
    let mut samples = Vec::with_capacity(total_samples);
    let freq = 440.0_f64;
    let amplitude = 0.5_f32;

    for frame in 0..num_frames {
        let t = frame as f64 / sample_rate as f64;
        let value = amplitude * (2.0 * std::f64::consts::PI * freq * t).sin() as f32;
        for _ in 0..channels {
            samples.push(value);
        }
    }

    AudioData {
        samples,
        sample_rate,
        channels,
    }
}

// ---------------------------------------------------------------------------
// Group 1: ops_by_duration — Pipeline operations vs. audio length
// ---------------------------------------------------------------------------
fn ops_by_duration(c: &mut Criterion) {
    let mut group = c.benchmark_group("ops_by_duration");

    for &(dur, label) in DURATIONS {
        let audio = generate_sine(dur, 44100, 2);
        let total_samples = audio.samples.len() as u64;
        group.throughput(Throughput::Elements(total_samples));

        // Volume
        group.bench_with_input(BenchmarkId::new("volume", label), &audio, |b, audio| {
            let op = VolumeOp::from_db(-6.0);
            b.iter_batched(
                || audio.clone(),
                |input| op.apply(input).unwrap(),
                BatchSize::LargeInput,
            );
        });

        // Normalize
        group.bench_with_input(BenchmarkId::new("normalize", label), &audio, |b, audio| {
            let op = NormalizeOp::new(-1.0);
            b.iter_batched(
                || audio.clone(),
                |input| op.apply(input).unwrap(),
                BatchSize::LargeInput,
            );
        });

        // Fade
        group.bench_with_input(BenchmarkId::new("fade", label), &audio, |b, audio| {
            let op = FadeOp::from_specs(Some(TimeSpec::Seconds(0.5)), Some(TimeSpec::Seconds(0.5)))
                .unwrap();
            b.iter_batched(
                || audio.clone(),
                |input| op.apply(input).unwrap(),
                BatchSize::LargeInput,
            );
        });

        // Trim (to half duration)
        let half = dur / 2.0;
        group.bench_with_input(BenchmarkId::new("trim", label), &audio, |b, audio| {
            let op = TrimOp::from_specs(TimeSpec::Seconds(0.0), Some(TimeSpec::Seconds(half)));
            b.iter_batched(
                || audio.clone(),
                |input| op.apply(input).unwrap(),
                BatchSize::LargeInput,
            );
        });

        // Channels (stereo → mono)
        group.bench_with_input(BenchmarkId::new("channels", label), &audio, |b, audio| {
            let op = ChannelsOp::new(ChannelMode::Mono);
            b.iter_batched(
                || audio.clone(),
                |input| op.apply(input).unwrap(),
                BatchSize::LargeInput,
            );
        });
    }

    group.finish();
}

// ---------------------------------------------------------------------------
// Group 2: resample — Resample operations (compute-intensive)
// ---------------------------------------------------------------------------
#[cfg(feature = "resample")]
fn resample(c: &mut Criterion) {
    let mut group = c.benchmark_group("resample");
    group.sample_size(10);

    let resample_durations: &[(f64, &str)] = &[(1.0, "1s"), (10.0, "10s")];

    let conversions: &[(u32, u32, &str)] = &[
        (44100, 48000, "44100_to_48000"),
        (48000, 44100, "48000_to_44100"),
        (44100, 22050, "44100_to_22050"),
    ];

    for &(dur, dur_label) in resample_durations {
        for &(src_rate, dst_rate, conv_label) in conversions {
            let audio = generate_sine(dur, src_rate, 2);
            let total_samples = audio.samples.len() as u64;
            let id = format!("{conv_label}/{dur_label}");

            group.throughput(Throughput::Elements(total_samples));
            group.bench_with_input(BenchmarkId::new("resample", &id), &audio, |b, audio| {
                let op = ResampleOp::new(dst_rate).unwrap();
                b.iter_batched(
                    || audio.clone(),
                    |input| op.apply(input).unwrap(),
                    BatchSize::LargeInput,
                );
            });
        }
    }

    group.finish();
}

// ---------------------------------------------------------------------------
// Group 3: channels — Channel mode comparison
// ---------------------------------------------------------------------------
fn channels(c: &mut Criterion) {
    let mut group = c.benchmark_group("channels");

    let stereo = generate_sine(10.0, 44100, 2);
    let mono = generate_sine(10.0, 44100, 1);

    group.bench_function("stereo_to_mono", |b| {
        let op = ChannelsOp::new(ChannelMode::Mono);
        b.iter_batched(
            || stereo.clone(),
            |input| op.apply(input).unwrap(),
            BatchSize::LargeInput,
        );
    });

    group.bench_function("mono_to_stereo", |b| {
        let op = ChannelsOp::new(ChannelMode::Stereo);
        b.iter_batched(
            || mono.clone(),
            |input| op.apply(input).unwrap(),
            BatchSize::LargeInput,
        );
    });

    group.bench_function("stereo_extract_left", |b| {
        let op = ChannelsOp::new(ChannelMode::Extract(ChannelSelector::Left));
        b.iter_batched(
            || stereo.clone(),
            |input| op.apply(input).unwrap(),
            BatchSize::LargeInput,
        );
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// Group 4: concat_split — Non-pipeline operations
// ---------------------------------------------------------------------------
fn concat_split(c: &mut Criterion) {
    let mut group = c.benchmark_group("concat_split");

    // Concat: 10 × 1s segments
    let segments: Vec<AudioData> = (0..10).map(|_| generate_sine(1.0, 44100, 2)).collect();
    group.bench_function("concat_10x1s", |b| {
        b.iter_batched(
            || segments.clone(),
            |segs| concat_audio(segs).unwrap(),
            BatchSize::LargeInput,
        );
    });

    // Split by count: 10s → 4 parts
    let audio_10s = generate_sine(10.0, 44100, 2);
    group.bench_function("split_count_4", |b| {
        b.iter(|| split_audio(&audio_10s, &SplitMode::Count(4)).unwrap());
    });

    // Split by duration: 10s → 2.5s chunks
    group.bench_function("split_duration_2_5s", |b| {
        b.iter(|| split_audio(&audio_10s, &SplitMode::Duration(TimeSpec::Seconds(2.5))).unwrap());
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// Group 5: codec_encode — Encoding benchmarks
// ---------------------------------------------------------------------------
fn codec_encode(c: &mut Criterion) {
    let mut group = c.benchmark_group("codec_encode");
    group.sample_size(10);

    let audio = generate_sine(5.0, 44100, 2);

    // WAV encoding (always available)
    {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("out.wav");
        group.bench_function("wav", |b| {
            b.iter(|| CodecRegistry::encode(&audio, &path, AudioFormat::Wav).unwrap());
        });
    }

    // MP3 encoding (feature-gated)
    #[cfg(feature = "mp3-enc")]
    {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("out.mp3");
        group.bench_function("mp3", |b| {
            b.iter(|| CodecRegistry::encode(&audio, &path, AudioFormat::Mp3).unwrap());
        });
    }

    // FLAC encoding (feature-gated)
    #[cfg(feature = "flac-enc")]
    {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("out.flac");
        group.bench_function("flac", |b| {
            b.iter(|| CodecRegistry::encode(&audio, &path, AudioFormat::Flac).unwrap());
        });
    }

    group.finish();
}

// ---------------------------------------------------------------------------
// Group 6: codec_decode — Decoding benchmarks
// ---------------------------------------------------------------------------
fn codec_decode(c: &mut Criterion) {
    let mut group = c.benchmark_group("codec_decode");
    group.sample_size(10);

    let audio = generate_sine(5.0, 44100, 2);

    // WAV decode
    let wav_dir = tempfile::tempdir().unwrap();
    let wav_path = wav_dir.path().join("test.wav");
    CodecRegistry::encode(&audio, &wav_path, AudioFormat::Wav).unwrap();
    group.bench_function("wav", |b| {
        b.iter(|| CodecRegistry::decode(&wav_path).unwrap());
    });

    // MP3 decode
    #[cfg(feature = "mp3-enc")]
    let (_mp3_dir, mp3_path) = {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.mp3");
        CodecRegistry::encode(&audio, &path, AudioFormat::Mp3).unwrap();
        (dir, path)
    };
    #[cfg(feature = "mp3-enc")]
    group.bench_function("mp3", |b| {
        b.iter(|| CodecRegistry::decode(&mp3_path).unwrap());
    });

    // FLAC decode
    #[cfg(feature = "flac-enc")]
    let (_flac_dir, flac_path) = {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.flac");
        CodecRegistry::encode(&audio, &path, AudioFormat::Flac).unwrap();
        (dir, path)
    };
    #[cfg(feature = "flac-enc")]
    group.bench_function("flac", |b| {
        b.iter(|| CodecRegistry::decode(&flac_path).unwrap());
    });

    group.finish();
}

#[cfg(feature = "resample")]
criterion_group!(
    benches,
    ops_by_duration,
    resample,
    channels,
    concat_split,
    codec_encode,
    codec_decode,
);

#[cfg(not(feature = "resample"))]
criterion_group!(
    benches,
    ops_by_duration,
    channels,
    concat_split,
    codec_encode,
    codec_decode,
);

criterion_main!(benches);
