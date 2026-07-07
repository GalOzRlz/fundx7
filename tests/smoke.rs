use fundsp::prelude64::{dc, An, BufferVec};
use std::io::Write;
use std::time::Duration;

mod common;

use common::generate_wav;
use fundx7::fm::voice::{Parameters, Voice};
use fundx7::{PatchBank};

#[test]
fn smoke_test() {

    let patch_bank_bytes =
        std::fs::read("star1-fast-decay.syx").expect("test file star1-fast-decay.syx not found");

    let patch_bank = PatchBank::new(&patch_bank_bytes);

    for (patch_idx, expected_algo, _expected_feedback) in [(0, 3, 0), (20, 2, 3), (31, 4, 7)] {
        assert_eq!(
            patch_bank.patches[patch_idx].algorithm, expected_algo,
            "parsed patch does not match expected algorithm, patch idx: {}, data: {:#?}",
            patch_idx, patch_bank.patches[patch_idx]
        );
    }

    for patch_number in 0..32 {
        let patch = patch_bank.patches[patch_number];

        let wav_data = generate_wav(patch, 60.0, 44100, std::time::Duration::from_secs(2));

        let file_name = format!("smoke-{}.wav", patch.name.iter().collect::<String>().trim());
        let mut file = std::fs::File::create(file_name).unwrap();
        file.write_all(&wav_data).unwrap();
        file.sync_all().unwrap();
    }
}
#[test]
fn dsp_test() {
    const SAMPLE_RATE: f32 = 44100.0;
    let patch_bank_bytes =
        std::fs::read("star1-fast-decay.syx").expect("test file star1-fast-decay.syx not found");

    let patch_bank = PatchBank::new(&patch_bank_bytes);
    let patch = patch_bank.patches[4];
    let parameters = Parameters {
        gate: true,
        sustain: false,
        velocity: 1.0,
        note: 40.0,
        ..Parameters::default()
    };
    let voice = Voice::new(patch, parameters, SAMPLE_RATE);
    //                           midi note 40 | gate on
    let mut synth = (dc(40.0) |  dc(1.0) ) >> An(voice);
    let mut input = BufferVec::new(2);
    let mut output = BufferVec::new(1);
    synth.process(64, &input.buffer_ref(), &mut output.buffer_mut());
    let out_slice = output.channel_f32(0);
    assert_ne!(input.channel_f32(0), out_slice);
    assert_eq!(out_slice.len(), 64);
    let gen_samples = patch.generate_samples(40.0, SAMPLE_RATE as u32, Duration::from_secs(2));
    assert_eq!(&gen_samples[0..64], out_slice);
}