// Copyright 2025 Tyler Neely (tylerneely@gmail.com).
// Copyright 2021 Emilie Gillet (emilie.o.gillet@gmail.com)
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
// THE SOFTWARE.
//
// See http://creativecommons.org/licenses/MIT/ for more information.

//! (mostly) Idiomatic Rust port of Mutable Instruments Plaits DX7/FM synthesis engine.
//!
//! This crate provides a port of the FM synthesis components from the
//! Mutable Instruments Plaits Eurorack module, focusing specifically on
//! the DX7-style FM synthesis engine.
//!
//! # Examples
//!
//! ```
//! use std::time::Duration;
//! use std::io::Write;
//!
//! // for WAV functionality
//! use hound::{WavSpec, WavWriter};
//!
//! use dx7::{PatchBank, Patch};
//!
//! fn generate_wav(patch: Patch, midi_note: f32, sample_rate: u32, duration: Duration) -> Vec<u8> {
//!     let buf = patch.generate_samples(midi_note, sample_rate, duration);
//!
//!     // Find peak amplitude for normalization
//!     let peak = buf.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
//!
//!     // Normalize to -1.0 to 1.0 range if needed, with headroom
//!     let normalize_factor = if peak > 0.8 { 0.8 / peak } else { 1.0 };
//!
//!     let wav_spec = WavSpec {
//!         channels: 1,
//!         sample_rate,
//!         bits_per_sample: 32,
//!         sample_format: hound::SampleFormat::Float,
//!     };
//!
//!     let mut ret = vec![];
//!     let mut cursor = std::io::Cursor::new(&mut ret);
//!
//!     let mut wav_writer = WavWriter::new(&mut cursor, wav_spec).unwrap();
//!
//!     for sample in &buf {
//!         wav_writer.write_sample(sample * normalize_factor).unwrap();
//!     }
//!
//!     wav_writer.finalize().unwrap();
//!
//!     ret
//! }
//!
//! fn main() {
//!     // read patch bank from the file in root
//!     let patch_bank_bytes = std::fs::read("star1-fast-decay.syx").unwrap();
//!
//!     let patch_bank = PatchBank::new(&patch_bank_bytes);
//!
//!     // patch 1 in most SYSEX librarians, ours is 0-indexed
//!     let my_favorite_patch = patch_bank.patches[0];
//!
//!
//!     // midi_note is based on midi note 60.0 correlating to C4 at 260hz. midi_note of 69.0 corresponds to
//!     // A4 at 437hz.
//!     let midi_note_c4 = 60.0;
//!     let sample_rate = 44100;
//!     let key_on_time = std::time::Duration::from_secs(2);
//!
//!     // generate our normalized wave samples
//!     let wav_data = generate_wav(my_favorite_patch, midi_note_c4, sample_rate, key_on_time);
//!
//!     let actually_write_file = false;
//!
//!     if actually_write_file {
//!         let file_name = format!("smoke-{}.wav", my_favorite_patch.name.iter().collect::<String>().trim());
//!         let mut file = std::fs::File::create(file_name).unwrap();
//!         file.write_all(&wav_data).unwrap();
//!         file.sync_all().unwrap();
//!     }
//! }
//! ```

#![warn(missing_docs)]

pub mod fm;
mod stmlib;

/// Sample rate used by the synthesis engine (in Hz)
pub const SAMPLE_RATE: f32 = 48000.0;

/// Maximum block size for audio processing
pub const MAX_BLOCK_SIZE: usize = MAX_BUFFER_SIZE;

/// Number of operators for DX7
const NUM_OPERATORS: usize = 6;

/// Number of algorithms for DX7;
const NUM_ALGORITHMS: usize = 32;

use fundsp::MAX_BUFFER_SIZE;
pub use fm::patch::{Patch, PatchBank};

use fm::lfo::Lfo;
use fm::voice::Parameters;
use fm::voice::Voice;

impl Patch {
    /// midi_note is based on midi note 60.0 correlating to C4 at 260hz. midi_note of 69.0 corresponds to
    /// A4 at 437hz.
    pub fn generate_samples(
        self,
        midi_note: f32,
        sample_rate: u32,
        duration: std::time::Duration,
    ) -> Vec<f32> {
        const MAX_BLOCK_SIZE: usize = 24;
        let n_samples = duration.as_millis() as usize * (sample_rate as usize / 1000) as usize;
        let silence_threshold = 0.0001f32;
        let silence_duration_samples = (sample_rate as usize * 100) / 1000; // 100ms

        let mut voice = Voice::new(self.clone(), sample_rate as f32);
        let mut lfo = Lfo::new();
        lfo.init(sample_rate as f32);
        lfo.set(&self.modulations);
        lfo.reset();

        let mut output = Vec::new();

        // Phase 1: Render with gate on for the requested duration
        let mut parameters = Parameters {
            gate: true,
            sustain: false,
            velocity: 1.0,
            note: midi_note,
            ..Parameters::default()
        };

        let mut remaining = n_samples;
        while remaining > 0 {
            let block_size = remaining.min(MAX_BLOCK_SIZE);

            // Step the LFO
            lfo.step(block_size as f32);

            // Apply LFO modulations to parameters
            voice.parameters.pitch_mod = lfo.pitch_mod();
            voice.parameters.amp_mod = lfo.amp_mod();

            voice.render_temp(&parameters, block_size);
            output.extend_from_slice(&voice.temp_buffer[..block_size]);
            remaining -= block_size;
        }

        // Phase 2: Turn gate off and render until 100ms of silence
        parameters.gate = false;
        let mut consecutive_silent_samples = 0;

        loop {
            // Step the LFO
            lfo.step(MAX_BLOCK_SIZE as f32);

            // Apply LFO modulations to parameters
            voice.parameters.pitch_mod = lfo.pitch_mod();
            voice.parameters.amp_mod = lfo.amp_mod();

            voice.render_temp(&parameters, MAX_BLOCK_SIZE);

            // Check for silence in the rendered output
            let rendered = &voice.temp_buffer[..MAX_BLOCK_SIZE];
            for &sample in rendered {
                if sample.abs() < silence_threshold {
                    consecutive_silent_samples += 1;
                } else {
                    consecutive_silent_samples = 0;
                }
            }

            output.extend_from_slice(rendered);

            // Check if we've accumulated enough silence
            if consecutive_silent_samples >= silence_duration_samples {
                // Truncate to end after the silence duration
                let truncate_to = output
                    .len()
                    .saturating_sub(consecutive_silent_samples - silence_duration_samples);
                output.truncate(truncate_to);
                return output;
            }

            // Safety limit: don't render more than 10 seconds total
            if output.len() > sample_rate as usize * 10 {
                break;
            }
        }

        output
    }
}
