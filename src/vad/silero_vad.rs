// Based on: https://github.com/snakers4/silero-vad
// Original work Copyright (c) 2020-present Silero Team
// Modified for Rust implementation
// Licensed under the MIT License

use ndarray::{Array1, Array2, Array3};
use ort::{Session, Value};
use std::path::Path;

use super::speech_segment::SpeechSegment;

const THRESHOLD_GAP: f32 = 0.15;
const SAMPLING_RATE_8K: i32 = 8000;
const SAMPLING_RATE_16K: i32 = 16000;

pub struct SileroVadDetector {
    session: Session,
    threshold: f32,
    neg_threshold: f32,
    sampling_rate: i32,
    window_size_sample: usize,
    min_speech_samples: f32,
    speech_pad_samples: f32,
    max_speech_samples: f32,
    min_silence_samples: f32,
    min_silence_samples_at_max_speech: f32,
    audio_length_samples: usize,

    // State variables
    state: Array3<f32>,
    context: Array2<f32>,
    last_sr: i32,
    last_batch_size: usize,
}

impl SileroVadDetector {
    pub fn new(
        model_path: impl AsRef<Path>,
        threshold: f32,
        sampling_rate: i32,
        min_speech_duration_ms: i32,
        max_speech_duration_seconds: f32,
        min_silence_duration_ms: i32,
        speech_pad_ms: i32,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        if sampling_rate != SAMPLING_RATE_8K && sampling_rate != SAMPLING_RATE_16K {
            return Err("Sampling rate not supported, only available for [8000, 16000]".into());
        }

        let session = Session::builder()?
            .with_intra_threads(1)?
            .commit_from_file(model_path)?;

        let window_size_sample = if sampling_rate == SAMPLING_RATE_16K { 512 } else { 256 };
        let min_speech_samples = sampling_rate as f32 * min_speech_duration_ms as f32 / 1000.0;
        let speech_pad_samples = sampling_rate as f32 * speech_pad_ms as f32 / 1000.0;
        let max_speech_samples = sampling_rate as f32 * max_speech_duration_seconds
            - window_size_sample as f32 - 2.0 * speech_pad_samples;
        let min_silence_samples = sampling_rate as f32 * min_silence_duration_ms as f32 / 1000.0;
        let min_silence_samples_at_max_speech = sampling_rate as f32 * 98.0 / 1000.0;

        let mut detector = Self {
            session,
            threshold,
            neg_threshold: threshold - THRESHOLD_GAP,
            sampling_rate,
            window_size_sample,
            min_speech_samples,
            speech_pad_samples,
            max_speech_samples,
            min_silence_samples,
            min_silence_samples_at_max_speech,
            audio_length_samples: 0,
            state: Array3::zeros((2, 1, 128)),
            context: Array2::zeros((0, 0)),
            last_sr: 0,
            last_batch_size: 0,
        };

        detector.reset_states();
        Ok(detector)
    }

    pub fn reset_states(&mut self) {
        self.state = Array3::zeros((2, 1, 128));
        self.context = Array2::zeros((0, 0));
        self.last_sr = 0;
        self.last_batch_size = 0;
    }

    pub fn get_speech_segments(&mut self, input: &[f32]) -> Result<Vec<SpeechSegment>, Box<dyn std::error::Error>> {
        self.reset_states();
        let mut speech_prob_list = Vec::new();
        self.audio_length_samples = input.len();

        let mut i = 0;
        while i < input.len() {
            let end = (i + self.window_size_sample).min(input.len());
            let mut buffer = vec![0.0f32; self.window_size_sample];
            buffer[..end - i].copy_from_slice(&input[i..end]);

            let speech_prob = self.call(&[buffer], self.sampling_rate)?[0];
            speech_prob_list.push(speech_prob);

            i += self.window_size_sample;
        }

        Ok(self.calculate_prob(&speech_prob_list))
    }

    fn call(&mut self, x: &[Vec<f32>], sr: i32) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
        let (x, sr) = self.validate_input(x, sr)?;
        let number_samples = if sr == 16000 { 512 } else { 256 };

        if x[0].len() != number_samples {
            return Err(format!(
                "Provided number of samples is {} (Supported values: 256 for 8000 sample rate, 512 for 16000)",
                x[0].len()
            ).into());
        }

        let batch_size = x.len();
        let context_size = if sr == 16000 { 64 } else { 32 };

        if self.last_batch_size == 0 {
            self.reset_states();
        }
        if self.last_sr != 0 && self.last_sr != sr {
            self.reset_states();
        }
        if self.last_batch_size != 0 && self.last_batch_size != batch_size {
            self.reset_states();
        }

        if self.context.is_empty() {
            self.context = Array2::zeros((batch_size, context_size));
        }

        let x = self.concatenate(&self.context.clone(), &x);

        // Prepare inputs
        let input_array: Vec<f32> = x.iter().flat_map(|row| row.iter().cloned()).collect();
        let input_shape = [x.len(), x[0].len()];
        let input_tensor = Array2::from_shape_vec(input_shape, input_array)?;

        let sr_tensor = Array1::from_vec(vec![sr as i64]);
        let state_tensor = self.state.clone();

        let inputs = vec![
            ("input", Value::from_array(input_tensor)?),
            ("sr", Value::from_array(sr_tensor)?),
            ("state", Value::from_array(state_tensor)?),
        ];

        let outputs = self.session.run(inputs)?;

        let output: Array2<f32> = outputs["output"].try_extract_tensor()?.to_owned().into_dimensionality()?;
        let new_state: Array3<f32> = outputs["stateN"].try_extract_tensor()?.to_owned().into_dimensionality()?;

        self.context = self.get_last_columns(&x, context_size);
        self.last_sr = sr;
        self.last_batch_size = batch_size;
        self.state = new_state;

        Ok(output.into_iter().collect())
    }

    fn validate_input(&self, mut x: &[Vec<f32>], mut sr: i32) -> Result<(Vec<Vec<f32>>, i32), Box<dyn std::error::Error>> {
        let mut x_vec = x.to_vec();

        if sr != 16000 && sr % 16000 == 0 {
            let step = (sr / 16000) as usize;
            let mut reduced_x = Vec::new();

            for row in &x_vec {
                let new_row: Vec<f32> = row.iter().step_by(step).cloned().collect();
                reduced_x.push(new_row);
            }

            x_vec = reduced_x;
            sr = 16000;
        }

        if sr != SAMPLING_RATE_8K && sr != SAMPLING_RATE_16K {
            return Err(format!(
                "Only supports sample rates {} or {} (or multiples of 16000)",
                SAMPLING_RATE_8K, SAMPLING_RATE_16K
            ).into());
        }

        if !x_vec.is_empty() && (sr as f32 / x_vec[0].len() as f32) > 31.25 {
            return Err("Input audio is too short".into());
        }

        Ok((x_vec, sr))
    }

    fn concatenate(&self, a: &Array2<f32>, b: &[Vec<f32>]) -> Vec<Vec<f32>> {
        if a.is_empty() {
            return b.to_vec();
        }

        let rows = a.nrows();
        let mut result = Vec::with_capacity(rows);

        for i in 0..rows {
            let mut row = a.row(i).to_vec();
            if i < b.len() {
                row.extend_from_slice(&b[i]);
            }
            result.push(row);
        }

        result
    }

    fn get_last_columns(&self, array: &[Vec<f32>], context_size: usize) -> Array2<f32> {
        let rows = array.len();
        let mut result = Array2::zeros((rows, context_size));

        for (i, row) in array.iter().enumerate() {
            let cols = row.len();
            if context_size <= cols {
                let start = cols - context_size;
                for (j, &val) in row[start..].iter().enumerate() {
                    result[[i, j]] = val;
                }
            }
        }

        result
    }

    fn calculate_prob(&self, speech_prob_list: &[f32]) -> Vec<SpeechSegment> {
        let mut result = Vec::new();
        let mut triggered = false;
        let mut temp_end = 0usize;
        let mut prev_end = 0usize;
        let mut next_start = 0usize;
        let mut segment_start = 0usize;
        let mut segment_end = 0usize;
        let mut is_detecting = false;

        for (i, &speech_prob) in speech_prob_list.iter().enumerate() {
            if speech_prob >= self.threshold && temp_end != 0 {
                temp_end = 0;
                if next_start < prev_end {
                    next_start = self.window_size_sample * i;
                }
            }

            if speech_prob >= self.threshold && !triggered {
                triggered = true;
                is_detecting = true;
                segment_start = self.window_size_sample * i;
                continue;
            }

            if triggered && (self.window_size_sample * i - segment_start) as f32 > self.max_speech_samples {
                if prev_end != 0 {
                    segment_end = prev_end;
                    result.push(SpeechSegment::from_offsets(segment_start, segment_end, self.sampling_rate));

                    if next_start < prev_end {
                        triggered = false;
                        is_detecting = false;
                    } else {
                        segment_start = next_start;
                        is_detecting = true;
                    }

                    prev_end = 0;
                    next_start = 0;
                    temp_end = 0;
                } else {
                    segment_end = self.window_size_sample * i;
                    result.push(SpeechSegment::from_offsets(segment_start, segment_end, self.sampling_rate));

                    prev_end = 0;
                    next_start = 0;
                    temp_end = 0;
                    triggered = false;
                    is_detecting = false;
                    continue;
                }
            }

            if speech_prob < self.neg_threshold && triggered {
                if temp_end == 0 {
                    temp_end = self.window_size_sample * i;
                }

                if (self.window_size_sample * i - temp_end) as f32 > self.min_silence_samples_at_max_speech {
                    prev_end = temp_end;
                }

                if (self.window_size_sample * i - temp_end) as f32 < self.min_silence_samples {
                    continue;
                } else {
                    segment_end = temp_end;
                    if (segment_end - segment_start) as f32 > self.min_speech_samples {
                        result.push(SpeechSegment::from_offsets(segment_start, segment_end, self.sampling_rate));
                    }

                    prev_end = 0;
                    next_start = 0;
                    temp_end = 0;
                    triggered = false;
                    is_detecting = false;
                    continue;
                }
            }
        }

        if is_detecting && (self.audio_length_samples - segment_start) as f32 > self.min_speech_samples {
            segment_end = self.audio_length_samples;
            result.push(SpeechSegment::from_offsets(segment_start, segment_end, self.sampling_rate));
        }

        // Apply speech padding
        for i in 0..result.len() {
            if i == 0 {
                result[i].start_offset = result[i].start_offset.saturating_sub(self.speech_pad_samples as usize);
            }

            if i != result.len() - 1 {
                let silence_duration = result[i + 1].start_offset - result[i].end_offset;
                if (silence_duration as f32) < 2.0 * self.speech_pad_samples {
                    result[i].end_offset += silence_duration / 2;
                    result[i + 1].start_offset = result[i + 1].start_offset.saturating_sub(silence_duration / 2);
                } else {
                    result[i].end_offset = (result[i].end_offset as f32 + self.speech_pad_samples).min(self.audio_length_samples as f32) as usize;
                    result[i + 1].start_offset = result[i + 1].start_offset.saturating_sub(self.speech_pad_samples as usize);
                }
            } else {
                result[i].end_offset = (result[i].end_offset as f32 + self.speech_pad_samples).min(self.audio_length_samples as f32) as usize;
            }

            // Recalculate seconds
            result[i].start_second = (result[i].start_offset as f32 / self.sampling_rate as f32 * 1000.0).floor() / 1000.0;
            result[i].end_second = (result[i].end_offset as f32 / self.sampling_rate as f32 * 1000.0).floor() / 1000.0;
        }

        self.merge_segments(result)
    }

    fn merge_segments(&self, mut segments: Vec<SpeechSegment>) -> Vec<SpeechSegment> {
        if segments.is_empty() {
            return segments;
        }

        if segments.len() == 1 {
            return segments;
        }

        segments.sort_by_key(|s| s.start_offset);

        let mut result = Vec::new();
        let mut left = segments[0].start_offset;
        let mut right = segments[0].end_offset;

        for segment in segments.iter().skip(1) {
            if segment.start_offset > right {
                result.push(SpeechSegment::from_offsets(left, right, self.sampling_rate));
                left = segment.start_offset;
                right = segment.end_offset;
            } else {
                right = right.max(segment.end_offset);
            }
        }

        result.push(SpeechSegment::from_offsets(left, right, self.sampling_rate));
        result
    }
}
