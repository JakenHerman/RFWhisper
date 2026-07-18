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
}

#[derive(Args)]
struct DenoiseArgs {
    #[arg(long, short)]
    input: PathBuf,
    #[arg(long, short)]
    output: PathBuf,
    #[arg(long, short, default_value = "deepfilternet3")]
    model: String,
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

fn cmd_denoise(args: DenoiseArgs) -> Result<(), String> {
    let (mono, sr) = bench::read_wav_mono(&args.input).map_err(|e| e.to_string())?;
    let model = resolve_model(&args.model);
    let mut eng = select_engine(&model).map_err(|e| e.to_string())?;
    let (y, stats) = eng.process_file(&mono, sr);

    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: sr,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let mut writer = hound::WavWriter::create(&args.output, spec).map_err(|e| e.to_string())?;
    for v in &y {
        writer.write_sample(*v).map_err(|e| e.to_string())?;
    }
    writer.finalize().map_err(|e| e.to_string())?;

    let rep = serde_json::json!({
        "model": model,
        "input": args.input.display().to_string(),
        "output": args.output.display().to_string(),
        "rtf": stats.rtf(),
        "seconds": stats.seconds_audio,
    });
    let rendered = serde_json::to_string_pretty(&rep).expect("serialize report");
    println!("{rendered}");
    if let Some(report) = args.report {
        std::fs::write(&report, &rendered).map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let result: Result<i32, String> = match cli.command {
        Command::Denoise(args) => cmd_denoise(args).map(|()| 0),
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
    };
    match result {
        Ok(code) => ExitCode::from(code.clamp(0, 255) as u8),
        Err(msg) => {
            eprintln!("error: {msg}");
            ExitCode::FAILURE
        }
    }
}
