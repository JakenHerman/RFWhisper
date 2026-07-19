//! `rfwhisper` CLI entrypoint (clap).

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Args, Parser, Subcommand};

use rfwhisper::bench;
use rfwhisper::constants::DEFAULT_BLOCKSIZE;
use rfwhisper::denoise::select_engine;
use rfwhisper::models::fetch;
use rfwhisper::realtime;
use rfwhisper::samples;

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
    /// List available signal kinds, noise kinds, and named presets.
    List,
}

/// Every generator `samples synth` can emit, clean and noise in one namespace.
///
/// `Speech` and `Impulses` are the pre-reconciliation names for `Ssb` and `Qrn`.
/// CONTRIBUTING tells contributors to paste `samples synth` invocations into PR
/// testing criteria, so those spellings stay valid rather than silently failing
/// in an already-written PR.
#[derive(clap::ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
enum SignalKind {
    /// Formant-model voice band-limited to the 300 Hz-2.7 kHz SSB passband.
    Ssb,
    /// Alias for `ssb`.
    Speech,
    /// Keyed 600 Hz CW ("CQ", 20 WPM, 5 ms raised-cosine edges).
    Cw,
    /// FT8-shaped 8-GFSK: 6.25 Hz tone spacing, 0.16 s symbols.
    Ft8,
    /// Recovered narrowband-FM audio (wider passband than SSB).
    VhfFm,
    /// Gaussian white noise.
    White,
    /// 60 Hz powerline buzz: 30 harmonics with per-harmonic wobble.
    Powerline,
    /// Solar/MPPT inverter switching: a ringing impulse train.
    Inverter,
    /// VDSL / PLC hash: band-limited noise plus residual carriers.
    Vdsl,
    /// Atmospheric-QRN static crashes (Poisson-timed).
    Qrn,
    /// Alias for `qrn`.
    Impulses,
    /// clean + noise mixed at --snr-db (see --clean / --noise / --preset).
    Mix,
}

impl SignalKind {
    /// The clean-signal generator this kind maps to, if it is one.
    fn as_signal(self) -> Option<samples::SignalKind> {
        match self {
            Self::Ssb | Self::Speech => Some(samples::SignalKind::Ssb),
            Self::Cw => Some(samples::SignalKind::Cw),
            Self::Ft8 => Some(samples::SignalKind::Ft8),
            Self::VhfFm => Some(samples::SignalKind::VhfFm),
            _ => None,
        }
    }

    /// The noise generator this kind maps to, if it is one.
    fn as_noise(self) -> Option<samples::NoiseKind> {
        match self {
            Self::White => Some(samples::NoiseKind::White),
            Self::Powerline => Some(samples::NoiseKind::Powerline),
            Self::Inverter => Some(samples::NoiseKind::Inverter),
            Self::Vdsl => Some(samples::NoiseKind::Vdsl),
            Self::Qrn | Self::Impulses => Some(samples::NoiseKind::Qrn),
            _ => None,
        }
    }
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
    #[arg(long, default_value_t = 0.0, allow_negative_numbers = true)]
    snr_db: f64,
    /// For --kind mix: also write the clean reference here (for denoise --reference).
    #[arg(long)]
    clean_out: Option<PathBuf>,
    /// A named clean/noise/SNR combination (implies --kind mix, overrides
    /// --clean / --noise / --snr-db). See `rfwhisper samples list`.
    #[arg(long)]
    preset: Option<String>,
}

fn cmd_samples_list() {
    println!("signals:  ssb (speech)  cw  ft8  vhf-fm");
    println!("noise:    powerline  inverter  vdsl  qrn (impulses)  white");
    println!(
        "
presets (--preset <name>, implies --kind mix):"
    );
    for p in &samples::PRESETS {
        println!(
            "  {:<22} {} + {} @ {:+.0} dB SNR",
            p.name,
            p.signal.as_str(),
            p.noise.as_str(),
            p.snr_db
        );
    }
}

fn cmd_samples_synth(args: SynthArgs) -> Result<(), String> {
    // A preset names the whole triple, so it implies mix and wins over the flags.
    let resolved = match &args.preset {
        Some(name) => Some(
            *samples::preset(name)
                .ok_or_else(|| format!("unknown preset {name:?}; run `rfwhisper samples list`"))?,
        ),
        None => None,
    };

    if resolved.is_some() || args.kind == SignalKind::Mix {
        let (sig, noise, snr) = match resolved {
            Some(p) => (p.signal, p.noise, p.snr_db),
            None => (
                args.clean
                    .as_signal()
                    .ok_or_else(|| format!("--clean {:?} is a noise kind", args.clean))?,
                args.noise
                    .as_noise()
                    .ok_or_else(|| format!("--noise {:?} is a signal kind", args.noise))?,
                args.snr_db,
            ),
        };
        let fx = samples::render_mix(sig, noise, snr, args.sr, args.seconds, args.seed)
            .map_err(|e| e.to_string())?;
        samples::write_wav(&args.out, &fx.noisy, fx.sr).map_err(|e| e.to_string())?;
        if let Some(clean_out) = &args.clean_out {
            samples::write_wav(clean_out, &fx.clean, fx.sr).map_err(|e| e.to_string())?;
            println!("wrote clean reference {}", clean_out.display());
        }
        println!(
            "wrote {} (mix: {} + {} @ {:+.1} dB, {:.1}s @ {} Hz, seed {})",
            args.out.display(),
            sig.as_str(),
            noise.as_str(),
            snr,
            args.seconds,
            args.sr,
            args.seed
        );
        if fx.headroom_scale < 1.0 {
            // Deeply negative SNRs push the mix well above full scale; both
            // channels are scaled together so the ratio is untouched.
            println!(
                "note: scaled both channels by {:.4} to fit headroom (SNR unchanged)",
                fx.headroom_scale
            );
        }
        return Ok(());
    }

    let signal = if let Some(sig) = args.kind.as_signal() {
        sig.render(args.sr, args.seconds, args.seed)
    } else if let Some(noise) = args.kind.as_noise() {
        noise.render(args.sr, args.seconds, args.seed)
    } else {
        return Err(format!("{:?} cannot be generated on its own", args.kind));
    }
    .map_err(|e| e.to_string())?;

    samples::write_wav(&args.out, &signal, args.sr).map_err(|e| e.to_string())?;
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
    /// Write a self-contained before/after HTML report to this path (#115).
    /// `report.json`'s `spectrogram_path` then points at it.
    #[arg(long)]
    spectrogram: Option<PathBuf>,
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

    let f64s = |v: &[f32]| v.iter().map(|x| *x as f64).collect::<Vec<f64>>();

    // Read the clean reference once; it feeds both the A1 gain and the HTML report.
    let clean = match &args.reference {
        None => None,
        Some(ref_path) => {
            let (clean, ref_sr) = read_mono_strict(ref_path)?;
            if ref_sr != sr {
                return Err((
                    EXIT_BAD_INPUT,
                    format!("reference sample rate {ref_sr} != input sample rate {sr}"),
                ));
            }
            Some(f64s(&clean))
        }
    };

    // snr_gain_db (A1): matched-filter gain of denoised vs noisy against the clean ref.
    let (noisy_f64, denoised_f64) = (f64s(&mono), f64s(&y));
    let snr_gain_db = match &clean {
        None => serde_json::Value::Null,
        Some(clean) => {
            let gain =
                rfwhisper::dsp::metrics::effective_snr_gain(clean, &noisy_f64, &denoised_f64, sr)
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

    // Self-contained before/after HTML report (#115).
    let spectrogram_path = match &args.spectrogram {
        None => serde_json::Value::Null,
        Some(html_path) => {
            let data = rfwhisper::report::build_report(
                &model,
                &noisy_f64,
                &denoised_f64,
                clean.as_deref(),
                sr,
            )
            .map_err(|e| (EXIT_BAD_INPUT, e.to_string()))?;
            rfwhisper::report::write_html(html_path, &data).map_err(|e| io_err(e.to_string()))?;
            serde_json::json!(html_path.display().to_string())
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
        "spectrogram_path": spectrogram_path,
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
        Command::Samples {
            command: SamplesCommand::List,
        } => {
            cmd_samples_list();
            Ok(0)
        }
    };
    match result {
        Ok(code) => ExitCode::from(code.clamp(0, 255) as u8),
        Err(msg) => {
            eprintln!("error: {msg}");
            ExitCode::FAILURE
        }
    }
}
