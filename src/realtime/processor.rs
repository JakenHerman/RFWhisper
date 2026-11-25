//! Real-time duplex audio (`rfwhisper denoise-live`, #13 / #14 / #17).
//!
//! Thread topology:
//!
//! ```text
//! input callback ──SPSC ring──▶ worker thread ──SPSC ring──▶ output callback
//!  (channel pick,                 StreamPipeline:              (mono fan-out
//!   no allocation)                resample → engine →           to every channel,
//!                                 A/B mix → resample            no allocation)
//!                                 + recording + telemetry
//! ```
//!
//! The audio callbacks only copy samples into / out of lock-free rings
//! (`rtrb`) and bump atomic counters — no allocation, no locks, no I/O, per
//! AGENTS.md § Hard Rules. All heavy work (inference, resampling, WAV writes,
//! telemetry) lives on the worker thread.
//!
//! Input and output are opened in each device's own default format (rate,
//! channel count, sample type), because real hardware pairs rarely agree — a
//! 44.1 kHz rig codec feeding a 48 kHz virtual cable is normal. The pipeline's
//! stateful resamplers bridge the rates without block-edge clicks.

use std::io::BufRead;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::sync_channel;
use std::sync::Arc;
use std::time::{Duration, Instant};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, Sample, SampleFormat, SizedSample};

use crate::constants::DEFAULT_BLOCKSIZE;
use crate::denoise::{select_engine, EngineError};
use crate::realtime::pipeline::StreamPipeline;

#[derive(Debug, thiserror::Error)]
pub enum RealtimeError {
    #[error("audio device error: {0}")]
    Device(String),
    #[error(transparent)]
    Engine(#[from] EngineError),
    #[error("recording error: {0}")]
    Record(String),
}

fn dev_err(e: impl std::fmt::Display) -> RealtimeError {
    RealtimeError::Device(e.to_string())
}

/// Options for [`stream_denoise`] (one per `denoise-live` flag).
#[derive(Debug, Clone)]
pub struct LiveOptions {
    /// Input device: index from `rfwhisper audio list`, or part of its name.
    pub input: Option<String>,
    /// Output device: index or part of its name (e.g. "CABLE", "BlackHole").
    pub output: Option<String>,
    pub model: String,
    /// Processing block in input-rate samples (the worker waits for this much).
    pub blocksize: usize,
    /// Which input channel carries the receive audio (0 = left / first).
    pub in_channel: usize,
    /// Start with the denoiser bypassed (A/B "raw" side).
    pub bypass: bool,
    /// Record the raw input (48 kHz float WAV, aligned with `record_denoised`).
    pub record_raw: Option<PathBuf>,
    /// Record the denoised output (48 kHz float WAV, aligned with `record_raw`).
    pub record_denoised: Option<PathBuf>,
    /// Stop automatically after this many seconds.
    pub duration: Option<f64>,
    /// Suppress the once-per-second telemetry line.
    pub quiet: bool,
}

impl Default for LiveOptions {
    fn default() -> Self {
        Self {
            input: None,
            output: None,
            model: "deepfilternet3".into(),
            blocksize: DEFAULT_BLOCKSIZE,
            in_channel: 0,
            bypass: false,
            record_raw: None,
            record_denoised: None,
            duration: None,
            quiet: false,
        }
    }
}

fn device_name(dev: &cpal::Device) -> String {
    dev.name().unwrap_or_else(|_| "<unknown>".into())
}

fn supports(dev: &cpal::Device, want_input: bool) -> bool {
    if want_input {
        dev.default_input_config().is_ok()
    } else {
        dev.default_output_config().is_ok()
    }
}

/// Resolve `--in` / `--out`: absent → system default; a number → the index
/// printed by `rfwhisper audio list`; anything else → the first device (with
/// the right direction) whose name contains it, case-insensitively.
fn resolve_device(
    host: &cpal::Host,
    spec: Option<&str>,
    want_input: bool,
) -> Result<cpal::Device, RealtimeError> {
    let dir = if want_input { "input" } else { "output" };
    let Some(spec) = spec else {
        let dev = if want_input {
            host.default_input_device()
        } else {
            host.default_output_device()
        };
        return dev.ok_or_else(|| {
            RealtimeError::Device(format!(
                "no default {dir} device; pick one with --{} (see `rfwhisper audio list`)",
                if want_input { "in" } else { "out" }
            ))
        });
    };
    let devices: Vec<cpal::Device> = host.devices().map_err(dev_err)?.collect();
    if let Ok(i) = spec.trim().parse::<usize>() {
        let dev = devices.into_iter().nth(i).ok_or_else(|| {
            RealtimeError::Device(format!(
                "no device with index {i}; run `rfwhisper audio list`"
            ))
        })?;
        if !supports(&dev, want_input) {
            return Err(RealtimeError::Device(format!(
                "device {i} ({}) has no {dir} channels; run `rfwhisper audio list`",
                device_name(&dev)
            )));
        }
        return Ok(dev);
    }
    let needle = spec.to_lowercase();
    devices
        .into_iter()
        .find(|d| supports(d, want_input) && device_name(d).to_lowercase().contains(&needle))
        .ok_or_else(|| {
            RealtimeError::Device(format!(
                "no {dir} device whose name contains {spec:?}; run `rfwhisper audio list`"
            ))
        })
}

/// Print the device table (`rfwhisper audio list`).
pub fn list_devices() -> Result<(), RealtimeError> {
    let host = cpal::default_host();
    let def_in = host.default_input_device().map(|d| device_name(&d));
    let def_out = host.default_output_device().map(|d| device_name(&d));
    println!("audio host: {}", host.id().name());
    let devices = host.devices().map_err(dev_err)?;
    let mut any = false;
    for (i, dev) in devices.enumerate() {
        any = true;
        let name = device_name(&dev);
        let inp = dev.default_input_config().ok();
        let out = dev.default_output_config().ok();
        let fmt = |c: &Option<cpal::SupportedStreamConfig>| {
            c.as_ref().map_or("-".to_string(), |c| {
                format!("{} ch @ {} Hz", c.channels(), c.sample_rate().0)
            })
        };
        let mut tags = Vec::new();
        if def_in.as_deref() == Some(name.as_str()) && inp.is_some() {
            tags.push("default in");
        }
        if def_out.as_deref() == Some(name.as_str()) && out.is_some() {
            tags.push("default out");
        }
        let tags = if tags.is_empty() {
            String::new()
        } else {
            format!("  [{}]", tags.join(", "))
        };
        println!(
            "{i:>3}  {name}\n       in: {}   out: {}{tags}",
            fmt(&inp),
            fmt(&out)
        );
    }
    if !any {
        println!("(no audio devices found)");
    }
    Ok(())
}

/// Counters shared between the callbacks, the worker, and the main thread.
#[derive(Default)]
struct Shared {
    stop: AtomicBool,
    bypass: AtomicBool,
    /// Input samples dropped because the worker fell behind.
    in_overflow: AtomicU64,
    /// Output callbacks that ran dry and played (partial) silence.
    out_underruns: AtomicU64,
    /// Output samples dropped to stop latency creeping up (clock drift).
    drift_drops: AtomicU64,
    /// Device-reported buffering, microseconds.
    in_dev_latency_us: AtomicU64,
    out_dev_latency_us: AtomicU64,
}

/// Try the requested fixed block size first, fall back to the device default —
/// WASAPI/CoreAudio frequently reject `Fixed` sizes the device didn't advertise.
fn with_buffer_fallback<T>(
    config: &cpal::StreamConfig,
    blocksize: usize,
    mut build: impl FnMut(&cpal::StreamConfig) -> Result<T, cpal::BuildStreamError>,
) -> Result<T, cpal::BuildStreamError> {
    let mut fixed = config.clone();
    fixed.buffer_size = cpal::BufferSize::Fixed(blocksize as u32);
    build(&fixed).or_else(|_| {
        let mut def = config.clone();
        def.buffer_size = cpal::BufferSize::Default;
        build(&def)
    })
}

/// Build the capture stream. Each attempt (fixed block size, then device
/// default) gets its own ring, because a failed build consumes the producer.
fn build_input_typed<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    blocksize: usize,
    channel: usize,
    ring_capacity: usize,
    shared: Arc<Shared>,
    debug: bool,
) -> Result<(cpal::Stream, rtrb::Consumer<f32>), cpal::BuildStreamError>
where
    T: SizedSample,
    f32: FromSample<T>,
{
    let channels = config.channels as usize;
    with_buffer_fallback(config, blocksize, |cfg| {
        let (mut prod, cons) = rtrb::RingBuffer::<f32>::new(ring_capacity);
        let sh = shared.clone();
        let stream = device.build_input_stream(
            cfg,
            move |data: &[T], info: &cpal::InputCallbackInfo| {
                let mut dropped = 0u64;
                for frame in data.chunks(channels) {
                    let v = f32::from_sample(frame[channel]);
                    if prod.push(v).is_err() {
                        dropped += 1;
                    }
                }
                if dropped > 0 {
                    sh.in_overflow.fetch_add(dropped, Ordering::Relaxed);
                }
                let ts = info.timestamp();
                if let Some(d) = ts.callback.duration_since(&ts.capture) {
                    sh.in_dev_latency_us
                        .store(d.as_micros() as u64, Ordering::Relaxed);
                }
            },
            move |err| {
                if debug {
                    eprintln!("input stream error: {err}");
                }
            },
            None,
        )?;
        Ok((stream, cons))
    })
}

fn build_output_typed<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    mut cons: rtrb::Consumer<f32>,
    shared: Arc<Shared>,
    debug: bool,
) -> Result<cpal::Stream, cpal::BuildStreamError>
where
    T: SizedSample + FromSample<f32>,
{
    let channels = config.channels as usize;
    device.build_output_stream(
        config,
        move |data: &mut [T], info: &cpal::OutputCallbackInfo| {
            let mut dry = false;
            for frame in data.chunks_mut(channels) {
                let v = match cons.pop() {
                    Ok(v) => v.clamp(-1.0, 1.0),
                    Err(_) => {
                        dry = true;
                        0.0
                    }
                };
                frame.fill(T::from_sample(v));
            }
            if dry {
                shared.out_underruns.fetch_add(1, Ordering::Relaxed);
            }
            let ts = info.timestamp();
            if let Some(d) = ts.playback.duration_since(&ts.callback) {
                shared
                    .out_dev_latency_us
                    .store(d.as_micros() as u64, Ordering::Relaxed);
            }
        },
        move |err| {
            if debug {
                eprintln!("output stream error: {err}");
            }
        },
        None,
    )
}

/// Open the capture stream: returns the stream, the consumer end of its ring,
/// and the device's rate and channel count.
fn open_input(
    device: &cpal::Device,
    blocksize: usize,
    channel: usize,
    shared: Arc<Shared>,
    debug: bool,
) -> Result<(cpal::Stream, rtrb::Consumer<f32>, u32, u16), RealtimeError> {
    let name = device_name(device);
    let supported = device
        .default_input_config()
        .map_err(|e| RealtimeError::Device(format!("cannot query input device {name:?}: {e}")))?;
    let config: cpal::StreamConfig = supported.config();
    if channel >= config.channels as usize {
        return Err(RealtimeError::Device(format!(
            "--in-channel {channel} but {name:?} has only {} channel(s) (counting from 0)",
            config.channels
        )));
    }
    // 250 ms of headroom; far more than any callback delivers at once.
    let cap = config.sample_rate.0 as usize / 4;
    let args = (device, &config, blocksize, channel, cap, shared, debug);
    let (stream, cons) = match supported.sample_format() {
        SampleFormat::F32 => {
            build_input_typed::<f32>(args.0, args.1, args.2, args.3, args.4, args.5, args.6)
        }
        SampleFormat::I16 => {
            build_input_typed::<i16>(args.0, args.1, args.2, args.3, args.4, args.5, args.6)
        }
        SampleFormat::U16 => {
            build_input_typed::<u16>(args.0, args.1, args.2, args.3, args.4, args.5, args.6)
        }
        SampleFormat::I32 => {
            build_input_typed::<i32>(args.0, args.1, args.2, args.3, args.4, args.5, args.6)
        }
        SampleFormat::I8 => {
            build_input_typed::<i8>(args.0, args.1, args.2, args.3, args.4, args.5, args.6)
        }
        SampleFormat::U8 => {
            build_input_typed::<u8>(args.0, args.1, args.2, args.3, args.4, args.5, args.6)
        }
        SampleFormat::F64 => {
            build_input_typed::<f64>(args.0, args.1, args.2, args.3, args.4, args.5, args.6)
        }
        other => {
            return Err(RealtimeError::Device(format!(
                "input device {name:?} uses unsupported sample format {other:?}"
            )))
        }
    }
    .map_err(|e| {
        RealtimeError::Device(format!(
            "could not open input device {name:?}: {e}. Another program may be using it, \
             or the OS may be blocking microphone access (macOS: System Settings → \
             Privacy & Security → Microphone)."
        ))
    })?;
    Ok((stream, cons, config.sample_rate.0, config.channels))
}

fn open_output(
    device: &cpal::Device,
    cons: rtrb::Consumer<f32>,
    shared: Arc<Shared>,
    debug: bool,
) -> Result<(cpal::Stream, u32, u16), RealtimeError> {
    let name = device_name(device);
    let supported = device
        .default_output_config()
        .map_err(|e| RealtimeError::Device(format!("cannot query output device {name:?}: {e}")))?;
    // Always the device-default buffer size: the ring decouples the output
    // block size from the processing block size, and WASAPI often rejects
    // `Fixed` sizes on output.
    let config: cpal::StreamConfig = supported.config();
    let stream = match supported.sample_format() {
        SampleFormat::F32 => build_output_typed::<f32>(device, &config, cons, shared, debug),
        SampleFormat::I16 => build_output_typed::<i16>(device, &config, cons, shared, debug),
        SampleFormat::U16 => build_output_typed::<u16>(device, &config, cons, shared, debug),
        SampleFormat::I32 => build_output_typed::<i32>(device, &config, cons, shared, debug),
        SampleFormat::I8 => build_output_typed::<i8>(device, &config, cons, shared, debug),
        SampleFormat::U8 => build_output_typed::<u8>(device, &config, cons, shared, debug),
        SampleFormat::F64 => build_output_typed::<f64>(device, &config, cons, shared, debug),
        other => {
            return Err(RealtimeError::Device(format!(
                "output device {name:?} uses unsupported sample format {other:?}"
            )))
        }
    }
    .map_err(|e| {
        RealtimeError::Device(format!(
            "could not open output device {name:?}: {e}. Another program may have it in \
             exclusive mode."
        ))
    })?;
    Ok((stream, config.sample_rate.0, config.channels))
}

type WavWriter = hound::WavWriter<std::io::BufWriter<std::fs::File>>;

fn create_wav(path: &PathBuf, sr: u32) -> Result<WavWriter, RealtimeError> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: sr,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    hound::WavWriter::create(path, spec)
        .map_err(|e| RealtimeError::Record(format!("cannot create {}: {e}", path.display())))
}

/// Per-second processing-time stats, plus a coarse all-session histogram.
struct ProcStats {
    window: Vec<f64>,
    window_audio_s: f64,
    window_wall_s: f64,
    /// 0.1 ms buckets up to 200 ms; last bucket is overflow.
    hist: Vec<u64>,
}

impl ProcStats {
    fn new() -> Self {
        Self {
            window: Vec::with_capacity(4096),
            window_audio_s: 0.0,
            window_wall_s: 0.0,
            hist: vec![0; 2001],
        }
    }

    fn record(&mut self, wall_ms: f64, audio_s: f64) {
        self.window.push(wall_ms);
        self.window_wall_s += wall_ms / 1000.0;
        self.window_audio_s += audio_s;
        let b = ((wall_ms * 10.0) as usize).min(self.hist.len() - 1);
        self.hist[b] += 1;
    }

    fn pct(sorted: &[f64], q: f64) -> f64 {
        if sorted.is_empty() {
            return 0.0;
        }
        sorted[((q * (sorted.len() - 1) as f64).round() as usize).min(sorted.len() - 1)]
    }

    /// (p50 ms, p99 ms, load fraction) for the window, then reset it.
    fn take_window(&mut self) -> (f64, f64, f64) {
        self.window.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let out = (
            Self::pct(&self.window, 0.5),
            Self::pct(&self.window, 0.99),
            if self.window_audio_s > 0.0 {
                self.window_wall_s / self.window_audio_s
            } else {
                0.0
            },
        );
        self.window.clear();
        self.window_audio_s = 0.0;
        self.window_wall_s = 0.0;
        out
    }

    fn session_pct(&self, q: f64) -> f64 {
        let total: u64 = self.hist.iter().sum();
        if total == 0 {
            return 0.0;
        }
        let want = (q * total as f64).ceil() as u64;
        let mut acc = 0;
        for (i, c) in self.hist.iter().enumerate() {
            acc += c;
            if acc >= want {
                return (i as f64 + 1.0) / 10.0;
            }
        }
        200.0
    }
}

/// Counters reset this long after the streams start (see `settled`).
const SETTLE_MS: u64 = 500;

/// What the worker reports back when it exits.
struct WorkerSummary {
    seconds: f64,
    proc_p50_ms: f64,
    proc_p99_ms: f64,
    latency_ms: f64,
}

/// Duplex stream: capture → [`StreamPipeline`] → playback, until Ctrl-C,
/// `q` + Enter, or `--duration` elapses.
pub fn stream_denoise(opts: &LiveOptions) -> Result<(), RealtimeError> {
    let blocksize = if opts.blocksize == 0 {
        DEFAULT_BLOCKSIZE
    } else {
        opts.blocksize
    };
    let debug = std::env::var("RFWHISPER_DEBUG").is_ok();

    let host = cpal::default_host();
    let input = resolve_device(&host, opts.input.as_deref(), true)?;
    let output = resolve_device(&host, opts.output.as_deref(), false)?;
    let in_name = device_name(&input);
    let out_name = device_name(&output);

    let shared = Arc::new(Shared::default());
    shared.bypass.store(opts.bypass, Ordering::Relaxed);

    // Ring sizes: 250 ms of headroom at either end is far more than any
    // callback delivers; latency is bounded separately by the drift guard.
    let in_sr_guess = input
        .default_input_config()
        .map(|c| c.sample_rate().0)
        .unwrap_or(48_000);
    let out_sr_guess = output
        .default_output_config()
        .map(|c| c.sample_rate().0)
        .unwrap_or(48_000);
    let (mut out_prod, out_cons) = rtrb::RingBuffer::<f32>::new(out_sr_guess as usize / 4);

    let (in_stream, mut in_cons, sr_in, in_channels) =
        open_input(&input, blocksize, opts.in_channel, shared.clone(), debug)?;
    let (out_stream, sr_out, out_channels) = open_output(&output, out_cons, shared.clone(), debug)?;

    // Recorders are created here so a bad path fails before audio starts.
    let mut rec_raw = opts
        .record_raw
        .as_ref()
        .map(|p| create_wav(p, crate::constants::NATIVE_DFN_SR_HZ))
        .transpose()?;
    let mut rec_den = opts
        .record_denoised
        .as_ref()
        .map(|p| create_wav(p, crate::constants::NATIVE_DFN_SR_HZ))
        .transpose()?;

    // Output priming / drift guard, in output samples.
    let prime = (sr_out as usize / 100).max(blocksize); // ~10 ms
    let high_water = prime + sr_out as usize * 40 / 1000; // +40 ms

    let model = opts.model.clone();
    let quiet = opts.quiet;
    let worker_shared = shared.clone();
    // The engine is built inside the worker: the DeepFilterNet3 backend is
    // `!Send` (tract's `Rc` state), so it cannot cross the spawn boundary.
    let (ready_tx, ready_rx) = sync_channel::<Result<(&'static str, f64), EngineError>>(1);
    let worker = std::thread::Builder::new()
        .name("rfwhisper-denoise".into())
        .spawn(move || -> Result<WorkerSummary, RealtimeError> {
            let engine = match select_engine(&model) {
                Ok(e) => e,
                Err(e) => {
                    let _ = ready_tx.send(Err(e));
                    return Ok(WorkerSummary {
                        seconds: 0.0,
                        proc_p50_ms: 0.0,
                        proc_p99_ms: 0.0,
                        latency_ms: 0.0,
                    });
                }
            };
            let mut pipe = StreamPipeline::new(engine, sr_in, sr_out);
            // Warm-up: the first inferences are slow (graph planning, caches).
            // Pay that before the streams start, not as an audible dropout.
            let mut scratch = Vec::with_capacity(sr_out as usize);
            pipe.process(&vec![0.0; sr_in as usize / 5], &mut scratch);
            scratch.clear();
            let _ = ready_tx.send(Ok((pipe.engine_name(), pipe.latency_ms())));

            for _ in 0..prime {
                let _ = out_prod.push(0.0);
            }

            let mut inbuf: Vec<f32> = Vec::with_capacity(in_sr_guess as usize / 4);
            let mut outbuf: Vec<f32> = Vec::with_capacity(out_sr_guess as usize / 4);
            let mut stats = ProcStats::new();
            let started = Instant::now();
            let mut last_report = Instant::now();
            let mut audio_s = 0.0f64;
            let mut lat_ms = pipe.latency_ms();
            // Startup transients (the output device starting a few ms after the
            // input, the drift guard trimming that initial backlog) are not
            // steady-state faults; counters restart once the stream settles.
            let mut settled = false;

            while !worker_shared.stop.load(Ordering::Relaxed) {
                let avail = in_cons.slots();
                if avail < blocksize {
                    std::thread::sleep(Duration::from_micros(500));
                } else {
                    inbuf.clear();
                    if let Ok(chunk) = in_cons.read_chunk(avail) {
                        let (a, b) = chunk.as_slices();
                        inbuf.extend_from_slice(a);
                        inbuf.extend_from_slice(b);
                        chunk.commit_all();
                    }
                    pipe.set_bypass(worker_shared.bypass.load(Ordering::Relaxed));
                    outbuf.clear();
                    let t0 = Instant::now();
                    pipe.process(&inbuf, &mut outbuf);
                    let wall_ms = t0.elapsed().as_secs_f64() * 1000.0;
                    let chunk_s = inbuf.len() as f64 / sr_in as f64;
                    stats.record(wall_ms, chunk_s);
                    audio_s += chunk_s;

                    if let Some(w) = rec_raw.as_mut() {
                        for v in pipe.last_raw() {
                            w.write_sample(*v)
                                .map_err(|e| RealtimeError::Record(e.to_string()))?;
                        }
                    }
                    if let Some(w) = rec_den.as_mut() {
                        for v in pipe.last_denoised() {
                            w.write_sample(*v)
                                .map_err(|e| RealtimeError::Record(e.to_string()))?;
                        }
                    }

                    // Drift guard: if the output ring is past the high-water
                    // mark (input clock faster than output clock), drop the
                    // excess so latency cannot creep up over a long session.
                    let fill = out_prod.buffer().capacity() - out_prod.slots();
                    let keep = if fill + outbuf.len() > high_water {
                        let excess = fill + outbuf.len() - prime;
                        let keep = outbuf.len().saturating_sub(excess);
                        worker_shared
                            .drift_drops
                            .fetch_add((outbuf.len() - keep) as u64, Ordering::Relaxed);
                        keep
                    } else {
                        outbuf.len()
                    };
                    for v in &outbuf[..keep] {
                        if out_prod.push(*v).is_err() {
                            break;
                        }
                    }
                }

                if !settled && started.elapsed() >= Duration::from_millis(SETTLE_MS) {
                    settled = true;
                    worker_shared.in_overflow.store(0, Ordering::Relaxed);
                    worker_shared.out_underruns.store(0, Ordering::Relaxed);
                    worker_shared.drift_drops.store(0, Ordering::Relaxed);
                }

                if last_report.elapsed() >= Duration::from_secs(1) {
                    last_report = Instant::now();
                    let (p50, p99, load) = stats.take_window();
                    let in_fill = in_cons.slots() as f64 / sr_in as f64;
                    let out_fill =
                        (out_prod.buffer().capacity() - out_prod.slots()) as f64 / sr_out as f64;
                    let dev_ms = (worker_shared.in_dev_latency_us.load(Ordering::Relaxed)
                        + worker_shared.out_dev_latency_us.load(Ordering::Relaxed))
                        as f64
                        / 1000.0;
                    let queue_ms = 1000.0 * (in_fill + out_fill + blocksize as f64 / sr_in as f64);
                    let dsp_ms = pipe.latency_ms() + p50;
                    lat_ms = dev_ms + queue_ms + dsp_ms;
                    if !quiet {
                        let mode = if pipe.bypass() { "BYPASS " } else { "DENOISE" };
                        eprintln!(
                            "[{:>5.0}s] {mode} {} | latency ~{lat_ms:.0} ms (devices {dev_ms:.0} \
                             + buffers {queue_ms:.0} + dsp {dsp_ms:.0}) | proc p50 {p50:.1} ms \
                             p99 {p99:.1} ms | load {:.0}% | in-drops {} underruns {} \
                             drift-drops {}",
                            started.elapsed().as_secs_f64(),
                            pipe.engine_name(),
                            load * 100.0,
                            worker_shared.in_overflow.load(Ordering::Relaxed),
                            worker_shared.out_underruns.load(Ordering::Relaxed),
                            worker_shared.drift_drops.load(Ordering::Relaxed),
                        );
                    }
                }
            }

            if let Some(w) = rec_raw.take() {
                w.finalize()
                    .map_err(|e| RealtimeError::Record(e.to_string()))?;
            }
            if let Some(w) = rec_den.take() {
                w.finalize()
                    .map_err(|e| RealtimeError::Record(e.to_string()))?;
            }
            Ok(WorkerSummary {
                seconds: audio_s,
                proc_p50_ms: stats.session_pct(0.5),
                proc_p99_ms: stats.session_pct(0.99),
                latency_ms: lat_ms,
            })
        })
        .expect("spawn worker thread");

    // Block until the worker has built its engine, so a bad model fails fast
    // here (streams never start) rather than silently playing through.
    let (engine_name, chain_ms) = match ready_rx.recv() {
        Ok(Ok(v)) => v,
        Ok(Err(e)) => {
            let _ = worker.join();
            return Err(RealtimeError::Engine(e));
        }
        Err(_) => {
            let _ = worker.join();
            return Err(RealtimeError::Device(
                "denoise worker failed to start".into(),
            ));
        }
    };

    in_stream.play().map_err(dev_err)?;
    out_stream.play().map_err(dev_err)?;

    eprintln!("rfwhisper denoise-live");
    eprintln!(
        "  input : {in_name}  ({sr_in} Hz, {in_channels} ch, using channel {})",
        opts.in_channel
    );
    eprintln!("  output: {out_name}  ({sr_out} Hz, {out_channels} ch)");
    eprintln!("  model : {engine_name}  (algorithmic latency {chain_ms:.1} ms)");
    if let Some(p) = &opts.record_raw {
        eprintln!("  record raw      → {}", p.display());
    }
    if let Some(p) = &opts.record_denoised {
        eprintln!("  record denoised → {}", p.display());
    }
    eprintln!("  controls: b + Enter = A/B bypass toggle · q + Enter or Ctrl-C = stop");

    // Ctrl-C / SIGTERM → stop. `set_handler` can only be installed once per
    // process; a second `denoise-live` in the same process (tests) just keeps
    // the first handler, which sets the same kind of flag.
    {
        let sh = shared.clone();
        let _ = ctrlc::set_handler(move || sh.stop.store(true, Ordering::Relaxed));
    }
    // Keyboard controls. EOF on stdin (running under a service manager with
    // stdin = /dev/null) just disables them; it does not stop the stream.
    {
        let sh = shared.clone();
        std::thread::Builder::new()
            .name("rfwhisper-keys".into())
            .spawn(move || {
                let stdin = std::io::stdin();
                for line in stdin.lock().lines() {
                    let Ok(line) = line else { break };
                    match line.trim().to_lowercase().as_str() {
                        "b" | "a" | "ab" => {
                            let now = !sh.bypass.load(Ordering::Relaxed);
                            sh.bypass.store(now, Ordering::Relaxed);
                            eprintln!("  → {}", if now { "BYPASS (raw audio)" } else { "DENOISE" });
                        }
                        "q" | "quit" | "exit" => {
                            sh.stop.store(true, Ordering::Relaxed);
                            break;
                        }
                        "" => {}
                        other => eprintln!("  (unknown command {other:?}; b = A/B, q = quit)"),
                    }
                }
            })
            .expect("spawn key thread");
    }

    let headless = std::env::var("CI").is_ok() || std::env::var("RFWHISPER_HEADLESS").is_ok();
    let duration = opts
        .duration
        .or(if headless { Some(0.2) } else { None })
        .map(Duration::from_secs_f64);
    let t0 = Instant::now();
    while !shared.stop.load(Ordering::Relaxed) {
        std::thread::sleep(Duration::from_millis(50));
        if duration.is_some_and(|d| t0.elapsed() >= d) {
            shared.stop.store(true, Ordering::Relaxed);
        }
    }

    drop(in_stream);
    drop(out_stream);
    let summary = worker
        .join()
        .map_err(|_| RealtimeError::Device("denoise worker panicked".into()))??;
    eprintln!(
        "stopped after {:.1} s of audio · proc p50 {:.1} ms p99 {:.1} ms · last latency \
         estimate ~{:.0} ms · in-drops {} · underruns {} · drift-drops {}",
        summary.seconds,
        summary.proc_p50_ms,
        summary.proc_p99_ms,
        summary.latency_ms,
        shared.in_overflow.load(Ordering::Relaxed),
        shared.out_underruns.load(Ordering::Relaxed),
        shared.drift_drops.load(Ordering::Relaxed),
    );
    if let Some(p) = &opts.record_raw {
        eprintln!("wrote {}", p.display());
    }
    if let Some(p) = &opts.record_denoised {
        eprintln!("wrote {}", p.display());
    }
    Ok(())
}
