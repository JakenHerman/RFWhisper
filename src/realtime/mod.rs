//! Realtime audio path (cpal-based duplex streaming).

pub mod processor;

pub use processor::{list_devices, stream_denoise, RealtimeError};
