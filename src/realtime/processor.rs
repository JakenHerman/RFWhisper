//! Real-time duplex stream: avoid heavy work in the audio callbacks — use a worker
//! thread (same topology as the Python/PortAudio original: callback enqueues,
//! worker denoises, output callback dequeues or emits silence).

use std::io::BufRead;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{sync_channel, TrySendError};
use std::sync::Arc;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

use crate::constants::DEFAULT_BLOCKSIZE;
use crate::denoise::{select_engine, EngineError};

#[derive(Debug, thiserror::Error)]
pub enum RealtimeError {
    #[error("audio device error: {0}")]
    Device(String),
    #[error(transparent)]
    Engine(#[from] EngineError),
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
            .map_err(|e| RealtimeError::Device(e.to_string()))?
            .nth(i)
            .ok_or_else(|| RealtimeError::Device(format!("no device with index {i}"))),
    }
}

/// Print the PortAudio-style device table (`rfwhisper audio list`).
pub fn list_devices() -> Result<(), RealtimeError> {
    let host = cpal::default_host();
    let devices = host
        .devices()
        .map_err(|e| RealtimeError::Device(e.to_string()))?;
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

/// Duplex stream: input callback enqueues; worker runs `DenoiseEngine::process`;
/// output callback dequeues (or plays silence when the worker is behind).
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

    let host = cpal::default_host();
    let input = device_by_index(&host, in_dev, true)?;
    let output = device_by_index(&host, out_dev, false)?;
    let dev_sr = input
        .default_input_config()
        .map_err(|e| RealtimeError::Device(e.to_string()))?
        .sample_rate();

    let config = cpal::StreamConfig {
        channels: 1,
        sample_rate: dev_sr,
        buffer_size: cpal::BufferSize::Fixed(blocksize as u32),
    };
    let sr = dev_sr.0;

    // Bounded queues, mirror the Python maxsize=32; drop on overflow rather than block.
    let (in_tx, in_rx) = sync_channel::<Vec<f32>>(32);
    let (out_tx, out_rx) = sync_channel::<Vec<f32>>(32);
    let stop = Arc::new(AtomicBool::new(false));

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
                let n = mono.len();
                let mut y = engine.process(&mono, sr);
                y.resize(n, 0.0);
                if !worker_stop.load(Ordering::Relaxed) {
                    match out_tx.try_send(y) {
                        Ok(()) | Err(TrySendError::Full(_)) => {}
                        Err(TrySendError::Disconnected(_)) => break,
                    }
                }
            }
        })
        .expect("spawn worker thread");

    let debug = std::env::var("RFWHISPER_DEBUG").is_ok();
    let in_stream = input
        .build_input_stream(
            &config,
            move |data: &[f32], _| {
                // try_send drops the block when the worker is behind (matches the
                // Python put_nowait/except-Full behaviour).
                let _ = in_tx.try_send(data.to_vec());
            },
            move |err| {
                if debug {
                    eprintln!("input stream error: {err}");
                }
            },
            None,
        )
        .map_err(|e| RealtimeError::Device(e.to_string()))?;

    let out_stream = output
        .build_output_stream(
            &config,
            move |data: &mut [f32], _| match out_rx.try_recv() {
                Ok(y) => {
                    let n = y.len().min(data.len());
                    data[..n].copy_from_slice(&y[..n]);
                    data[n..].fill(0.0);
                }
                Err(_) => data.fill(0.0),
            },
            move |err| {
                if debug {
                    eprintln!("output stream error: {err}");
                }
            },
            None,
        )
        .map_err(|e| RealtimeError::Device(e.to_string()))?;

    in_stream
        .play()
        .map_err(|e| RealtimeError::Device(e.to_string()))?;
    out_stream
        .play()
        .map_err(|e| RealtimeError::Device(e.to_string()))?;

    println!("Streaming (SR={sr} Hz, block={blocksize}); Ctrl+C or Enter to stop.");
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
