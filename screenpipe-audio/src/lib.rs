mod core;
mod multilingual;
pub mod pcm_decode;
pub mod stt;
pub use core::{
    default_input_device, default_output_device, list_audio_devices, parse_audio_device,
    record_and_transcribe, AudioDevice, AudioTranscriptionEngine, DeviceControl,
};
pub use pcm_decode::pcm_decode;
pub use stt::{create_whisper_channel, stt, AudioInput, TranscriptionResult, WhisperModel};
