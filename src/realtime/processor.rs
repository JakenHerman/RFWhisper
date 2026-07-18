//! Real-time duplex stream: avoid heavy work in the audio callbacks — use a worker
//! thread (callback enqueues, worker denoises, output callback dequeues or emits
//! silence).
//!
//! Input and output streams are configured independently from each device's own
//! default config (sample rate, channel count, sample format): real hardware pairs
//! rarely agree (e.g. a 16 kHz mono headset mic feeding 48 kHz stereo headphones).
//! The worker resamples between the two rates; the output callback up-mixes mono
//! to every output channel and decouples block sizes through a small carry buffer.

use std::io::BufRead;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{sync_channel, Receiver, SyncSender, TrySendError};
use std::sync::Arc;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::SampleFormat;

use crate::constants::DEFAULT_BLOCKSIZE;
use crate::denoise::{select_engine, EngineError};
use crate::dsp::resample::to_native_rate;

#[derive(Debug, thiserror::Error)]
pub enum RealtimeError {
    #[error("audio device error: {0}")]
    Device(String),
    #[error(transparent)]
    Engine(#[from] EngineError),
}

fn dev_err(e: impl std::fmt::Display) -> RealtimeError {
    RealtimeError::Device(e.to_string())
}

fn device_by_index(
    host: &cpal::Host,
    index: Option<usize>,
    want_input: bool,
) -> Result<cpal::Device, RealtimeError> {
    match index {
        None => {
            let dev = if want_input {
                host.default_input_device()
            } else {
                host.default_output_device()
            };
            dev.ok_or_else(|| RealtimeError::Device("no default device".into()))
        }
        Some(i) => host
            .devices()
            .map_err(dev_err)?
            .nth(i)
            .ok_or_else(|| RealtimeError::Device(format!("no device with index {i}"))),
    }
}

/// Print the device table (`rfwhisper audio list`).
pub fn list_devices() -> Result<(), RealtimeError> {
    let host = cpal::default_host();
    let devices = host.devices().map_err(dev_err)?;
    for (i, dev) in devices.enumerate() {
        let name = dev.name().unwrap_or_else(|_| "<unknown>".into());
        let in_ch = dev
            .default_input_config()
            .map(|c| c.channels())
            .unwrap_or(0);
        let out_ch = dev
            .default_output_config()
            .map(|c| c.channels())
            .unwrap_or(0);
        let sr = dev
            .default_input_config()
            .or_else(|_| dev.default_output_config())
            .map(|c| c.sample_rate().0)
            .unwrap_or(0);
        println!("{i:>3} {name}  (in: {in_ch} ch, out: {out_ch} ch, {sr} Hz)");
    }
    Ok(())
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

/// Build the capture stream in the device's native format; the callback extracts
/// channel 0 as f32 and enqueues one mono chunk per callback (drop on overflow).
fn build_input_stream(
    device: &cpal::Device,
    blocksize: usize,
    tx: SyncSender<Vec<f32>>,
    debug: bool,
) -> Result<(cpal::Stream, u32), RealtimeError> {
    let supported = device.default_input_config().map_err(dev_err)?;
    let sr = supported.sample_rate().0;
    let channels = supported.channels() as usize;
    let config: cpal::StreamConfig = supported.config();
    let err_cb = move |err| {
        if debug {
            eprintln!("input stream error: {err}");
        }
    };

    let stream = match supported.sample_format() {
        SampleFormat::F32 => with_buffer_fallback(&config, blocksize, |cfg| {
            let tx = tx.clone();
            device.build_input_stream(
                cfg,
                move |data: &[f32], _| {
                    let mono: Vec<f32> = data.iter().step_by(channels).copied().collect();
                    let _ = tx.try_send(mono);
                },
                err_cb,
                None,
            )
        }),
        SampleFormat::I16 => with_buffer_fallback(&config, blocksize, |cfg| {
            let tx = tx.clone();
            device.build_input_stream(
                cfg,
                move |data: &[i16], _| {
                    let mono: Vec<f32> = data
                        .iter()
                        .step_by(channels)
                        .map(|v| *v as f32 / 32_768.0)
                        .collect();
                    let _ = tx.try_send(mono);
                },
                err_cb,
                None,
            )
        }),
        SampleFormat::U16 => with_buffer_fallback(&config, blocksize, |cfg| {
            let tx = tx.clone();
            device.build_input_stream(
                cfg,
                move |data: &[u16], _| {
                    let mono: Vec<f32> = data
                        .iter()
                        .step_by(channels)
                        .map(|v| (*v as f32 - 32_768.0) / 32_768.0)
                        .collect();
                    let _ = tx.try_send(mono);
                },
                err_cb,
                None,
            )
        }),
        other => {
            return Err(RealtimeError::Device(format!(
                "unsupported input sample format {other:?}"
            )))
        }
    }
    .map_err(dev_err)?;
    Ok((stream, sr))
}

/// Pulls processed mono chunks and writes them across every output channel,
/// carrying leftovers between callbacks so input/output block sizes need not match.
struct MonoFanOut {
    rx: Receiver<Vec<f32>>,
    carry: Vec<f32>,
    pos: usize,
}

impl MonoFanOut {
    fn next_sample(&mut self) -> f32 {
        if self.pos >= self.carry.len() {
            match self.rx.try_recv() {
                Ok(chunk) => {
                    self.carry = chunk;
                    self.pos = 0;
                }
                Err(_) => return 0.0,
            }
            if self.carry.is_empty() {
                return 0.0;
            }
        }
        let v = self.carry[self.pos];
        self.pos += 1;
        v
    }
}

/// Build the playback stream in the device's native format. Always uses the
/// device-default buffer size: the carry buffer in [`MonoFanOut`] decouples the
/// output block size from the processing block size, so there is nothing to gain
/// from forcing `Fixed` here (and WASAPI frequently rejects it).
fn build_output_stream(
    device: &cpal::Device,
    rx: Receiver<Vec<f32>>,
    debug: bool,
) -> Result<(cpal::Stream, u32), RealtimeError> {
    let supported = device.default_output_config().map_err(dev_err)?;
    let sr = supported.sample_rate().0;
    let channels = supported.channels() as usize;
    let config: cpal::StreamConfig = supported.config();
    let err_cb = move |err| {
        if debug {
            eprintln!("output stream error: {err}");
        }
    };

    let stream = match supported.sample_format() {
        SampleFormat::F32 => {
            let mut fan = MonoFanOut {
                rx,
                carry: Vec::new(),
                pos: 0,
            };
            device.build_output_stream(
                &config,
                move |data: &mut [f32], _| {
                    for frame in data.chunks_mut(channels) {
                        let v = fan.next_sample();
                        frame.fill(v);
                    }
                },
                err_cb,
                None,
            )
        }
        SampleFormat::I16 => {
            let mut fan = MonoFanOut {
                rx,
                carry: Vec::new(),
                pos: 0,
            };
            device.build_output_stream(
                &config,
                move |data: &mut [i16], _| {
                    for frame in data.chunks_mut(channels) {
                        let v = (fan.next_sample().clamp(-1.0, 1.0) * 32_767.0) as i16;
                        frame.fill(v);
                    }
                },
                err_cb,
                None,
            )
        }
        other => {
            return Err(RealtimeError::Device(format!(
                "unsupported output sample format {other:?}"
            )))
        }
    }
    .map_err(dev_err)?;
    Ok((stream, sr))
}

/// Duplex stream: input callback enqueues; worker runs `DenoiseEngine::process`
/// and resamples input-rate → output-rate; output callback dequeues (or plays
/// silence when the worker is behind).
pub fn stream_denoise(
    in_dev: Option<usize>,
    out_dev: Option<usize>,
    model: &str,
    blocksize: usize,
) -> Result<(), RealtimeError> {
    let mut engine = select_engine(model)?;
    let blocksize = if blocksize == 0 {
        DEFAULT_BLOCKSIZE
    } else {
        blocksize
    };
    let debug = std::env::var("RFWHISPER_DEBUG").is_ok();

    let host = cpal::default_host();
    let input = device_by_index(&host, in_dev, true)?;
    let output = device_by_index(&host, out_dev, false)?;

    // Bounded queues; drop on overflow rather than block the audio threads.
    let (in_tx, in_rx) = sync_channel::<Vec<f32>>(32);
    let (out_tx, out_rx) = sync_channel::<Vec<f32>>(32);
    let stop = Arc::new(AtomicBool::new(false));

    let (in_stream, sr_in) = build_input_stream(&input, blocksize, in_tx, debug)?;
    let (out_stream, sr_out) = build_output_stream(&output, out_rx, debug)?;

    let worker_stop = stop.clone();
    let worker = std::thread::Builder::new()
        .name("rfwhisper-denoise".into())
        .spawn(move || {
            while !worker_stop.load(Ordering::Relaxed) {
                let mono = match in_rx.recv_timeout(std::time::Duration::from_millis(200)) {
                    Ok(m) => m,
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => continue,
                    Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
                };
                let y = engine.process(&mono, sr_in);
                let y = if sr_in == sr_out {
                    y
                } else {
                    to_native_rate(&y, sr_in, sr_out)
                };
                if !worker_stop.load(Ordering::Relaxed) {
                    match out_tx.try_send(y) {
                        Ok(()) | Err(TrySendError::Full(_)) => {}
                        Err(TrySendError::Disconnected(_)) => break,
                    }
                }
            }
        })
        .expect("spawn worker thread");

    in_stream.play().map_err(dev_err)?;
    out_stream.play().map_err(dev_err)?;

    println!(
        "Streaming (in {sr_in} Hz -> out {sr_out} Hz, block={blocksize}); Ctrl+C or Enter to stop."
    );
    if std::env::var("CI").is_ok() || std::env::var("RFWHISPER_HEADLESS").is_ok() {
        std::thread::sleep(std::time::Duration::from_millis(200));
    } else {
        let mut line = String::new();
        let _ = std::io::stdin().lock().read_line(&mut line);
    }

    stop.store(true, Ordering::Relaxed);
    drop(in_stream);
    drop(out_stream);
    let _ = worker.join();
    Ok(())
}
