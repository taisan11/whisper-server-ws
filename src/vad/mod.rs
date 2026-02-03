// Based on: https://github.com/snakers4/silero-vad
// Original work Copyright (c) 2020-present Silero Team
// Modified for Rust implementation
// Licensed under the MIT License

pub mod silero_vad;
pub mod speech_segment;

pub use silero_vad::SileroVadDetector;
pub use speech_segment::SpeechSegment;
