# fundx7

Pure Rust fundx7 emulator - now supported as a FunDSP node!

Forked as a part of the Nabi synth project: https://github.com/GalOzRlz/nabi_core

Used with FunDSP:
```rust
use fundx7::fm::voice::{Parameters, Voice};
use fundx7::{PatchBank, SAMPLE_RATE};
use fundsp::prelude64::{dc, An, BufferVec};
use hound::{WavSpec, WavWriter};

fn render_sample() {
    let patch_bank_bytes =
        std::fs::read("star1-fast-decay.syx").expect("test file star1-fast-decay.syx not found");

    let patch_bank = PatchBank::new(&patch_bank_bytes);
    let patch = patch_bank.patches[4];
    let voice = Voice::new(patch, Parameters::default(), SAMPLE_RATE);
    //           midi note 40 | gate on
    let mut synth = (dc(40.0) |  dc(1.0) ) >> An(voice);
    let mut input = BufferVec::new(2);
    let mut output = BufferVec::new(2);
    synth.process(64, &input.buffer_ref(), &mut output.buffer_mut());


    let spec = WavSpec {
        channels: 1,
        sample_rate: SAMPLE_RATE as u32,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };

    let mut writer = WavWriter::create("sample.wav", spec).unwrap();

    for &sample in output.channel_f32(0) {
        writer.write_sample(sample).unwrap();
    }
}
```

Offline usage is the same:

```rust
use std::time::Duration;
use std::io::Write;

// for WAV functionality
use hound::{WavSpec, WavWriter};

use fundx7::{PatchBank, Patch};

fn generate_wav(patch: Patch, midi_note: f32, sample_rate: u32, duration: Duration) -> Vec<u8> {
    let buf = patch.generate_samples(midi_note, sample_rate, duration);

    // Find peak amplitude for normalization
    let peak = buf.iter().map(|s| s.abs()).fold(0.0f32, f32::max);

    // Normalize to -1.0 to 1.0 range if needed, with headroom
    let normalize_factor = if peak > 0.8 { 0.8 / peak } else { 1.0 };

    let wav_spec = WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };

    let mut ret = vec![];
    let mut cursor = std::io::Cursor::new(&mut ret);

    let mut wav_writer = WavWriter::new(&mut cursor, wav_spec).unwrap();

    for sample in &buf {
        wav_writer.write_sample(sample * normalize_factor).unwrap();
    }

    wav_writer.finalize().unwrap();

    ret
}

fn main() {
    // read patch bank from the file in root
    let patch_bank_bytes = std::fs::read("star1-fast-decay.syx").unwrap();

    let patch_bank = PatchBank::new(&patch_bank_bytes);

    // patch 1 in most SYSEX librarians, ours is 0-indexed
    let my_favorite_patch = patch_bank.patches[0];


    // midi_note is based on midi note 60.0 correlating to C4 at 260hz. midi_note of 69.0 corresponds to
    // A4 at 437hz.
    let midi_note_c4 = 60.0;
    let sample_rate = 44100;
    let key_on_time = std::time::Duration::from_secs(2);

    // generate our normalized wave samples
    let wav_data = generate_wav(my_favorite_patch, midi_note_c4, sample_rate, key_on_time);

    let actually_write_file = false;

    if actually_write_file {
        let file_name = format!("smoke-{}.wav", my_favorite_patch.name.iter().collect::<String>().trim());
        let mut file = std::fs::File::create(file_name).unwrap();
        file.write_all(&wav_data).unwrap();
        file.sync_all().unwrap();
    }
}

```
