// Based on: https://github.com/snakers4/silero-vad
// Original work Copyright (c) 2020-present Silero Team
// Modified for Rust implementation
// Licensed under the MIT License

#[derive(Debug, Clone)]
pub struct SpeechSegment {
    pub start_offset: usize,
    pub end_offset: usize,
    pub start_second: f32,
    pub end_second: f32,
}

impl SpeechSegment {
    pub fn new(start_offset: usize, end_offset: usize, start_second: f32, end_second: f32) -> Self {
        Self {
            start_offset,
            end_offset,
            start_second,
            end_second,
        }
    }

    pub fn from_offsets(start_offset: usize, end_offset: usize, sampling_rate: i32) -> Self {
        let start_second = calculate_second_by_offset(start_offset, sampling_rate);
        let end_second = calculate_second_by_offset(end_offset, sampling_rate);
        Self::new(start_offset, end_offset, start_second, end_second)
    }
}

fn calculate_second_by_offset(offset: usize, sampling_rate: i32) -> f32 {
    let second_value = offset as f32 / sampling_rate as f32;
    (second_value * 1000.0).floor() / 1000.0
}
