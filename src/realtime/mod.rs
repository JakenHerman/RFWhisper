//! Realtime audio path: device-independent [`pipeline`] plus cpal duplex I/O
//! ([`processor`]).

pub mod pipeline;
pub mod processor;

pub use processor::{list_devices, stream_denoise, LiveOptions, RealtimeError};
