use fundx7::fm::patch::{OpEnvelope, Operator, Patch};
use fundx7::fm::voice::{Parameters, Voice};

#[test]
fn test_envelope_triggering() {
    let mut patch = Patch::default();

    // Create a simple operator with fast attack, no decay/sustain drop, instant release
    let operator = Operator {
        envelope: OpEnvelope {
            rate: [99, 99, 99, 99],   // Fast everything
            level: [99, 99, 99, 0],    // Full level, then to 0 on release
        },
        level: 99,
        coarse: 1,
        fine: 0,
        ..Operator::default()
    };

    patch.set_op(1, operator);
    patch.algorithm = 31; // Simple algorithm with one carrier

    let sample_rate = 44100;
    let params_off = Parameters {
        gate: false,
        sustain: false,
        velocity: 1.0,
        note: 69.0,
        ..Parameters::default()
    };
    let mut voice = Voice::new(patch, params_off, sample_rate as f32);
    voice.render_temp( 64);
    let max_off = voice.temp_buffer.iter().take(64).map(|x| x.abs()).fold(0.0f32, f32::max);
    println!("Max amplitude with gate OFF: {}", max_off);

    // Render with gate ON (should trigger envelope)
    voice.parameters = Parameters {
            gate: true,
            sustain: false,
            velocity: 1.0,
            note: 69.0,
            ..Parameters::default()
        };
    voice.render_temp(64);
    let max_on = voice.temp_buffer.iter().take(64).map(|x| x.abs()).fold(0.0f32, f32::max);
    println!("Max amplitude with gate ON: {}", max_on);

    // Check envelope level directly
    println!("Operator 0 level after gate ON: {}", voice.op_level(0));

    // The gate ON render should have significantly more amplitude
    assert!(max_on > 0.01, "Gate ON should produce audible output, got {}", max_on);
    assert!(max_on > max_off * 10.0, "Gate ON should be much louder than gate OFF");
}
