//! `rfwhisper` CLI entrypoint (clap).

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Args, Parser, Subcommand};

use rfwhisper::bench;
use rfwhisper::constants::DEFAULT_BLOCKSIZE;
use rfwhisper::denoise::select_engine;
use rfwhisper::models::fetch;
use rfwhisper::realtime;

#[derive(Parser)]
#[command(
    name = "rfwhisper",
    about = "Real-time AI denoising for ham radio (local-first, GPLv3+)",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Offline WAV denoise (float PCM; resamples to model rate internally).
    Denoise(DenoiseArgs),
    /// Real-time: input device index -> output device (virtual cable).
    DenoiseLive(DenoiseLiveArgs),
    /// PortAudio-style device helpers.
    Audio {
        #[command(subcommand)]
        command: AudioCommand,
    },
    /// GUI status (native GUI is planned; see the roadmap).
    Gui,
    /// Model download and verification.
    Models {
        #[command(subcommand)]
        command: ModelsCommand,
    },
    /// Benchmark helpers (A4/A5).
    Bench {
        #[command(subcommand)]
        command: BenchCommand,
    },
    /// Deterministic synthetic test signals (refs #19).
    Samples {
        #[command(subcommand)]
        command: SamplesCommand,
    },
}

#[derive(Subcommand)]
enum SamplesCommand {
    /// Generate a test WAV (seeded — same flags always produce the same file).
    Synth(SynthArgs),
}

#[derive(clap::ValueEnum, Clone, Copy, Debug)]
enum SignalKind {
    /// AM-modulated tone stack standing in for SSB speech.
    Speech,
    /// Keyed 600 Hz CW ("CQ", 5 ms raised-cosine edges).
    Cw,
    /// Gaussian white noise.
    White,
    /// 60 Hz powerline buzz + harmonics + hash.
    Powerline,
    /// Atmospheric-QRN static crashes.
    Impulses,
    /// clean + noise mixed at --snr-db (see --clean / --noise).
    Mix,
}

#[derive(Args)]
struct SynthArgs {
    #[arg(long, value_enum)]
    kind: SignalKind,
    #[arg(long, short)]
    out: PathBuf,
    #[arg(long, default_value_t = 5.0)]
    seconds: f64,
    #[arg(long, default_value_t = 48_000)]
    sr: u32,
    #[arg(long, default_value_t = 7)]
    seed: u64,
    /// For --kind mix: the clean component.
    #[arg(long, value_enum, default_value = "speech")]
    clean: SignalKind,
    /// For --kind mix: the noise component.
    #[arg(long, value_enum, default_value = "powerline")]
    noise: SignalKind,
    /// For --kind mix: target SNR of clean vs noise, in dB.
    #[arg(long, default_value_t = 0.0)]
    snr_db: f64,
    /// For --kind mix: also write the clean reference here (for denoise --reference).
    #[arg(long)]
    clean_out: Option<PathBuf>,
}

fn synth_signal(kind: SignalKind, n: usize, sr: u32, seed: u64) -> Result<Vec<f32>, String> {
    use rfwhisper::samples;
    Ok(match kind {
        SignalKind::Speech => samples::speech_like(n, sr, seed),
        SignalKind::Cw => samples::cw(n, sr, 20),
        SignalKind::White => samples::white(n, seed),
        SignalKind::Powerline => samples::powerline(n, sr, seed),
        SignalKind::Impulses => samples::impulses(n, sr, seed),
        SignalKind::Mix => return Err("mix cannot be a mix component".into()),
    })
}

fn cmd_samples_synth(args: SynthArgs) -> Result<(), String> {
    use rfwhisper::samples;
    let n = (args.seconds * args.sr as f64) as usize;
    let signal = match args.kind {
        SignalKind::Mix => {
            let clean = synth_signal(args.clean, n, args.sr, args.seed)?;
            let noise = synth_signal(args.noise, n, args.sr, args.seed.wrapping_add(1))?;
            if let Some(clean_out) = &args.clean_out {
                samples::write_wav(clean_out, &clean, args.sr)?;
                println!("wrote clean reference {}", clean_out.display());
            }
            samples::mix_at_snr(&clean, &noise, args.snr_db)
        }
        kind => synth_signal(kind, n, args.sr, args.seed)?,
    };
    samples::write_wav(&args.out, &signal, args.sr)?;
    println!(
        "wrote {} ({:?}, {:.1}s @ {} Hz, seed {})",
        args.out.display(),
        args.kind,
        args.seconds,
        args.sr,
        args.seed
    );
    Ok(())
}

#[derive(Args)]
struct DenoiseArgs {
    #[arg(long, short)]
    input: PathBuf,
    #[arg(long, short)]
    output: PathBuf,
    #[arg(long, short, default_value = "deepfilternet3")]
    model: String,
    /// Clean reference WAV; when given, the report includes snr_gain_db (A1).
    #[arg(long)]
    reference: Option<PathBuf>,
    #[arg(long)]
    report: Option<PathBuf>,
}

#[derive(Args)]
struct DenoiseLiveArgs {
    /// Input device index (default input device when omitted).
    #[arg(long = "in")]
    in_dev: Option<usize>,
    /// Output device index (default output device when omitted).
    #[arg(long = "out")]
    out_dev: Option<usize>,
    #[arg(long, short, default_value = "deepfilternet3")]
    model: String,
    #[arg(long, default_value_t = DEFAULT_BLOCKSIZE)]
    blocksize: usize,
}

#[derive(Subcommand)]
enum AudioCommand {
    /// List audio devices.
    List,
}

#[derive(Subcommand)]
enum ModelsCommand {
    /// Download pre-converted models with optional SHA-256 verification.
    Fetch {
        /// Do not download.
        #[arg(long)]
        no_network: bool,
        /// Verify on-disk SHAs only.
        #[arg(long)]
        verify_only: bool,
    },
}

#[derive(Subcommand)]
enum BenchCommand {
    /// Latency probes (offline / processing budget).
    Latency {
        path: PathBuf,
        #[arg(long, default_value = "spectral_stub")]
        model: String,
        #[arg(long, default_value_t = 480)]
        block: usize,
    },
    /// RTF (wall/audio) on a WAV file.
    Rtf {
        path: PathBuf,
        #[arg(long, default_value = "spectral_stub")]
        model: String,
    },
}

fn resolve_model(name: &str) -> String {
    if std::env::var("RFWHISPER_FORCE_STUB").as_deref() == Ok("1") {
        "spectral_stub".to_string()
    } else {
        name.to_string()
    }
}

/// Exit codes per issue #16: 0 success, 2 unsupported input, 3 model load failure.
const EXIT_BAD_INPUT: u8 = 2;
const EXIT_MODEL_FAILURE: u8 = 3;

fn read_mono_strict(path: &std::path::Path) -> Result<(Vec<f32>, u32), (u8, String)> {
    let reader = hound::WavReader::open(path).map_err(|e| {
        (
            EXIT_BAD_INPUT,
            format!("cannot read {}: {e}", path.display()),
        )
    })?;
    if reader.spec().channels != 1 {
        return Err((
            EXIT_BAD_INPUT,
            format!(
                "{} has {} channels; rfwhisper denoise expects mono — split channels first",
                path.display(),
                reader.spec().channels
            ),
        ));
    }
    drop(reader);
    bench::read_wav_mono(path).map_err(|e| (EXIT_BAD_INPUT, e.to_string()))
}

fn cmd_denoise(args: DenoiseArgs) -> Result<(), (u8, String)> {
    let (mono, sr) = read_mono_strict(&args.input)?;
    let model = resolve_model(&args.model);
    let mut eng = select_engine(&model).map_err(|e| (EXIT_MODEL_FAILURE, e.to_string()))?;
    let (y, stats) = eng.process_file(&mono, sr);

    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: sr,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let io_err = |e: String| (1u8, e);
    let mut writer =
        hound::WavWriter::create(&args.output, spec).map_err(|e| io_err(e.to_string()))?;
    for v in &y {
        writer.write_sample(*v).map_err(|e| io_err(e.to_string()))?;
    }
    writer.finalize().map_err(|e| io_err(e.to_string()))?;

    // snr_gain_db (A1): matched-filter gain of denoised vs noisy against the clean ref.
    let snr_gain_db = match &args.reference {
        None => serde_json::Value::Null,
        Some(ref_path) => {
            let (clean, ref_sr) = read_mono_strict(ref_path)?;
            if ref_sr != sr {
                return Err((
                    EXIT_BAD_INPUT,
                    format!("reference sample rate {ref_sr} != input sample rate {sr}"),
                ));
            }
            let f64s = |v: &[f32]| v.iter().map(|x| *x as f64).collect::<Vec<f64>>();
            let gain = rfwhisper::dsp::metrics::effective_snr_gain(
                &f64s(&clean),
                &f64s(&mono),
                &f64s(&y),
                sr,
            )
            .map_err(|e| (EXIT_BAD_INPUT, e.to_string()))?;
            // JSON has no inf/nan; the sentinels degrade to null with a note on stdout.
            if gain.is_finite() {
                serde_json::json!(gain)
            } else {
                eprintln!("note: snr gain sentinel ({gain}); reported as null");
                serde_json::Value::Null
            }
        }
    };

    let rep = serde_json::json!({
        "model": model,
        "input": args.input.display().to_string(),
        "output": args.output.display().to_string(),
        "sr": sr,
        "duration_s": stats.seconds_audio,
        "inference_time_ms": stats.wall_seconds * 1000.0,
        "rtf": stats.rtf(),
        "snr_gain_db": snr_gain_db,
        "spectrogram_path": serde_json::Value::Null,
    });
    let rendered = serde_json::to_string_pretty(&rep).expect("serialize report");
    println!("{rendered}");
    if let Some(report) = args.report {
        std::fs::write(&report, &rendered).map_err(|e| io_err(e.to_string()))?;
    }
    Ok(())
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    if let Command::Denoise(args) = cli.command {
        return match cmd_denoise(args) {
            Ok(()) => ExitCode::SUCCESS,
            Err((code, msg)) => {
                eprintln!("error: {msg}");
                ExitCode::from(code)
            }
        };
    }
    let result: Result<i32, String> = match cli.command {
        Command::Denoise(_) => unreachable!("handled above"),
        Command::DenoiseLive(args) => realtime::stream_denoise(
            args.in_dev,
            args.out_dev,
            &resolve_model(&args.model),
            args.blocksize,
        )
        .map(|()| 0)
        .map_err(|e| e.to_string()),
        Command::Audio {
            command: AudioCommand::List,
        } => realtime::list_devices()
            .map(|()| 0)
            .map_err(|e| e.to_string()),
        Command::Gui => {
            println!(
                "GUI: use `rfwhisper denoise-live` for v0.1; a native GUI is planned (v0.4). \
                 See rfwhisper.org for virtual cable setup."
            );
            Ok(0)
        }
        Command::Models {
            command:
                ModelsCommand::Fetch {
                    no_network,
                    verify_only,
                },
        } => Ok(fetch::run(no_network, verify_only)),
        Command::Bench { command } => match command {
            BenchCommand::Latency { path, model, block } => {
                bench::measure_file_latency(&path, &model, block)
                    .map(|r| {
                        println!(
                            "p50={:.2} ms p99={:.2} ms n={} (processing only)",
                            r.p50_ms, r.p99_ms, r.n_chunks
                        );
                        if r.p99_ms > 30.0 && model == "deepfilternet3" {
                            println!(
                                "Note: A4 is end-to-end < 100 ms p99; this is chunk \
                                 processing only."
                            );
                        }
                        0
                    })
                    .map_err(|e| e.to_string())
            }
            BenchCommand::Rtf { path, model } => bench::measure_file_rtf(&path, &model)
                .map(|(rtf, seconds)| {
                    println!("RTF={rtf:.4} (A5: < 0.5 on reference CPU) seconds={seconds:.2}");
                    0
                })
                .map_err(|e| e.to_string()),
        },
        Command::Samples {
            command: SamplesCommand::Synth(args),
        } => cmd_samples_synth(args).map(|()| 0),
    };
    match result {
        Ok(code) => ExitCode::from(code.clamp(0, 255) as u8),
        Err(msg) => {
            eprintln!("error: {msg}");
            ExitCode::FAILURE
        }
    }
}
