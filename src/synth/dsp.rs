#[cfg_attr(test, allow(unused_imports))] // std shadows these intrinsics in test builds
use micromath::F32Ext; // provides f32 methods (sin, powf, ...) in no_std

/// Waveform types for oscillators
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Waveform {
    Sine,
    Square,
    Sawtooth,
    Triangle,
    Pulse { width: f32 },
    Noise,
}

/// Single oscillator voice
pub struct Oscillator {
    phase: f32,
    phase_increment: f32,
    waveform: Waveform,
    sample_rate: f32,
    detune: f32,
    base_frequency: f32,
}

impl Oscillator {
    pub fn new(sample_rate: u32) -> Self {
        Self {
            phase: 0.0,
            phase_increment: 0.0,
            waveform: Waveform::Sawtooth,
            sample_rate: sample_rate as f32,
            detune: 1.0,
            base_frequency: 0.0,
        }
    }

    /// Set the base frequency in Hz. The configured detune ratio is applied
    /// on top, so the rendered frequency is `freq * detune`.
    pub fn set_frequency(&mut self, freq: f32) {
        self.base_frequency = freq;
        self.phase_increment = (freq * self.detune) / self.sample_rate;
    }

    /// Set the detune as a frequency ratio (1.0 = unison, 1.02 ~ +34 cents).
    /// Re-applies to the current base frequency immediately.
    pub fn set_detune(&mut self, detune: f32) {
        self.detune = detune.clamp(0.25, 4.0);
        self.set_frequency(self.base_frequency);
    }

    pub fn get_detune(&self) -> f32 {
        self.detune
    }

    pub fn set_waveform(&mut self, waveform: Waveform) {
        self.waveform = waveform;
    }

    pub fn render(&mut self) -> f32 {
        let sample = match self.waveform {
            Waveform::Sine => self.render_sine(),
            Waveform::Square => self.render_square(),
            Waveform::Sawtooth => self.render_sawtooth(),
            Waveform::Triangle => self.render_triangle(),
            Waveform::Pulse { width } => self.render_pulse(width),
            Waveform::Noise => self.render_noise(),
        };

        self.phase += self.phase_increment;
        while self.phase >= 1.0 {
            self.phase -= 1.0;
        }

        sample
    }

    fn render_sine(&self) -> f32 {
        (self.phase * 2.0 * core::f32::consts::PI).sin()
    }

    fn render_square(&self) -> f32 {
        if self.phase < 0.5 {
            1.0
        } else {
            -1.0
        }
    }

    fn render_sawtooth(&self) -> f32 {
        self.phase * 2.0 - 1.0
    }

    fn render_triangle(&self) -> f32 {
        if self.phase < 0.25 {
            self.phase * 4.0
        } else if self.phase < 0.75 {
            1.0 - (self.phase - 0.25) * 4.0
        } else {
            -1.0 + (self.phase - 0.75) * 4.0
        }
    }

    fn render_pulse(&self, width: f32) -> f32 {
        if self.phase < width {
            1.0
        } else {
            -1.0
        }
    }

    fn render_noise(&self) -> f32 {
        // Simple pseudo-random using LCG
        let seed = (self.phase * 1_000_000.0) as u32;
        let random = seed.wrapping_mul(1103515245).wrapping_add(12345);
        (random as f32 / u32::MAX as f32) * 2.0 - 1.0
    }
}

/// ADSR envelope generator
pub struct ADSR {
    attack: f32,
    decay: f32,
    sustain: f32,
    release: f32,
    state: ADSRState,
    value: f32,
    phase: f32,
    release_start: f32,
    sample_rate: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ADSRState {
    Idle,
    Attack,
    Decay,
    Sustain,
    Release,
}

impl ADSR {
    pub fn new(sample_rate: u32) -> Self {
        Self {
            attack: 0.01,
            decay: 0.3,
            sustain: 0.7,
            release: 0.5,
            state: ADSRState::Idle,
            value: 0.0,
            phase: 0.0,
            release_start: 0.0,
            sample_rate: sample_rate as f32,
        }
    }

    pub fn set_params(&mut self, attack: f32, decay: f32, sustain: f32, release: f32) {
        self.attack = attack.max(0.001);
        self.decay = decay.max(0.001);
        self.sustain = sustain.clamp(0.0, 1.0);
        self.release = release.max(0.001);
    }

    pub fn trigger(&mut self) {
        self.state = ADSRState::Attack;
        self.phase = 0.0;
    }

    pub fn release(&mut self) {
        if self.state != ADSRState::Idle {
            self.state = ADSRState::Release;
            self.phase = 0.0;
            // Release from the current envelope value, not the sustain level,
            // so a note released mid-attack/decay has no amplitude jump.
            self.release_start = self.value;
        }
    }

    pub fn render(&mut self) -> f32 {
        match self.state {
            ADSRState::Idle => {
                self.value = 0.0;
            }
            ADSRState::Attack => {
                self.phase += 1.0 / (self.attack * self.sample_rate);
                if self.phase >= 1.0 {
                    self.phase = 0.0;
                    self.value = 1.0;
                    self.state = ADSRState::Decay;
                } else {
                    self.value = self.phase;
                }
            }
            ADSRState::Decay => {
                self.phase += 1.0 / (self.decay * self.sample_rate);
                if self.phase >= 1.0 {
                    self.phase = 0.0;
                    self.value = self.sustain;
                    self.state = ADSRState::Sustain;
                } else {
                    self.value = 1.0 - (1.0 - self.sustain) * self.phase;
                }
            }
            ADSRState::Sustain => {
                self.value = self.sustain;
            }
            ADSRState::Release => {
                self.phase += 1.0 / (self.release * self.sample_rate);
                if self.phase >= 1.0 {
                    self.phase = 0.0;
                    self.value = 0.0;
                    self.state = ADSRState::Idle;
                } else {
                    self.value = self.release_start * (1.0 - self.phase);
                }
            }
        }

        self.value
    }

    pub fn is_active(&self) -> bool {
        self.state != ADSRState::Idle
    }

    pub fn get_value(&self) -> f32 {
        self.value
    }
}

/// Low-pass filter with resonance
pub struct LowPassFilter {
    cutoff: f32,
    resonance: f32,
    a0: f32,
    a1: f32,
    a2: f32,
    b1: f32,
    b2: f32,
    x1: f32,
    x2: f32,
    y1: f32,
    y2: f32,
    sample_rate: f32,
}

impl LowPassFilter {
    pub fn new(sample_rate: u32) -> Self {
        let mut filter = Self {
            cutoff: 1000.0,
            resonance: 0.0,
            a0: 1.0,
            a1: 0.0,
            a2: 0.0,
            b1: 0.0,
            b2: 0.0,
            x1: 0.0,
            x2: 0.0,
            y1: 0.0,
            y2: 0.0,
            sample_rate: sample_rate as f32,
        };
        filter.recalculate_coefficients();
        filter
    }

    pub fn set_cutoff(&mut self, cutoff: f32) {
        self.cutoff = cutoff.clamp(20.0, self.sample_rate / 2.0);
        self.recalculate_coefficients();
    }

    pub fn set_resonance(&mut self, resonance: f32) {
        // Clamp below 1.0: the biquad coefficient formula divides by
        // (1.0 - resonance), so resonance == 1.0 would produce inf/NaN
        // coefficients and permanently poison the filter state.
        self.resonance = resonance.clamp(0.0, 0.95);
        self.recalculate_coefficients();
    }

    fn recalculate_coefficients(&mut self) {
        let w = 2.0 * core::f32::consts::PI * self.cutoff / self.sample_rate;
        let cos_w = w.cos();
        let sin_w = w.sin();
        let alpha = sin_w / (2.0 * (1.0 - self.resonance));

        self.a0 = (1.0 - cos_w) / 2.0;
        self.a1 = 1.0 - cos_w;
        self.a2 = (1.0 - cos_w) / 2.0;
        self.b1 = -2.0 * cos_w;
        self.b2 = 1.0 - alpha;

        let a0_inv = 1.0 / (1.0 + alpha);
        self.a0 *= a0_inv;
        self.a1 *= a0_inv;
        self.a2 *= a0_inv;
        self.b1 *= a0_inv;
        self.b2 *= a0_inv;
    }

    pub fn process(&mut self, input: f32) -> f32 {
        let output = self.a0 * input + self.a1 * self.x1 + self.a2 * self.x2
            - self.b1 * self.y1
            - self.b2 * self.y2;

        // Defensive: if anything ever drives the state non-finite, flush it
        // instead of emitting NaN for the rest of the voice's life.
        let output = if output.is_finite() {
            output
        } else {
            self.reset();
            0.0
        };

        self.x2 = self.x1;
        self.x1 = input;
        self.y2 = self.y1;
        self.y1 = output;

        output
    }

    pub fn reset(&mut self) {
        self.x1 = 0.0;
        self.x2 = 0.0;
        self.y1 = 0.0;
        self.y2 = 0.0;
    }
}

/// Simple delay effect
pub struct Delay {
    buffer: [f32; 8192],
    write_pos: usize,
    delay_samples: usize,
    feedback: f32,
    mix: f32,
}

impl Default for Delay {
    fn default() -> Self {
        Self::new()
    }
}

impl Delay {
    pub fn new() -> Self {
        Self {
            buffer: [0.0; 8192],
            write_pos: 0,
            delay_samples: 2205, // ~50ms at 44.1kHz
            feedback: 0.3,
            mix: 0.3,
        }
    }

    pub fn set_delay_ms(&mut self, delay_ms: f32, sample_rate: u32) {
        self.delay_samples = ((delay_ms / 1000.0) * sample_rate as f32) as usize;
        self.delay_samples = self.delay_samples.min(self.buffer.len() - 1);
    }

    pub fn set_feedback(&mut self, feedback: f32) {
        self.feedback = feedback.clamp(0.0, 0.95);
    }

    pub fn set_mix(&mut self, mix: f32) {
        self.mix = mix.clamp(0.0, 1.0);
    }

    pub fn process(&mut self, input: f32) -> f32 {
        let read_pos = if self.write_pos >= self.delay_samples {
            self.write_pos - self.delay_samples
        } else {
            self.buffer.len() - self.delay_samples + self.write_pos
        };

        let delayed = self.buffer[read_pos];
        let output = input + delayed * self.feedback;

        self.buffer[self.write_pos] = output;
        self.write_pos = (self.write_pos + 1) % self.buffer.len();

        input * (1.0 - self.mix) + delayed * self.mix
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SR: u32 = 48_000;

    /// Deterministic pseudo-white-noise source for filter torture tests.
    struct Noise(u32);
    impl Noise {
        fn next(&mut self) -> f32 {
            self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            ((self.0 >> 8) as f32 / (1u32 << 24) as f32) * 2.0 - 1.0
        }
    }

    #[test]
    fn oscillator_output_bounded_all_waveforms() {
        let waveforms = [
            Waveform::Sine,
            Waveform::Square,
            Waveform::Sawtooth,
            Waveform::Triangle,
            Waveform::Pulse { width: 0.1 },
            Waveform::Pulse { width: 0.5 },
            Waveform::Pulse { width: 0.9 },
            Waveform::Noise,
        ];
        let freqs = [20.0_f32, 440.0, 2000.0, 8000.0, 15_000.0];

        for waveform in waveforms {
            for freq in freqs {
                let mut osc = Oscillator::new(SR);
                osc.set_waveform(waveform);
                osc.set_frequency(freq);
                for i in 0..4096 {
                    let s = osc.render();
                    assert!(
                        s.is_finite(),
                        "non-finite sample from {:?} at {} Hz (sample {})",
                        waveform,
                        freq,
                        i
                    );
                    assert!(
                        (-1.0..=1.0).contains(&s),
                        "out-of-bounds sample {} from {:?} at {} Hz",
                        s,
                        waveform,
                        freq
                    );
                }
            }
        }
    }

    #[test]
    fn oscillator_detune_spread() {
        let mut a = Oscillator::new(SR);
        let mut b = Oscillator::new(SR);
        a.set_waveform(Waveform::Sawtooth);
        b.set_waveform(Waveform::Sawtooth);
        a.set_frequency(220.0);
        b.set_frequency(220.0);
        b.set_detune(1.02); // same ratio the Bass patch uses for osc2

        assert_eq!(a.get_detune(), 1.0);
        assert!((b.get_detune() - 1.02).abs() < 1e-6);

        // Over time the two oscillators must diverge audibly.
        let mut divergence = 0.0_f32;
        for _ in 0..4800 {
            divergence += (a.render() - b.render()).abs();
        }
        assert!(
            divergence > 100.0,
            "detuned oscillator did not diverge (total divergence {})",
            divergence
        );
    }

    #[test]
    fn adsr_full_stage_cycle() {
        // 1 kHz sample rate keeps sample counts small and exact.
        let mut env = ADSR::new(1000);
        env.set_params(0.01, 0.01, 0.5, 0.01);

        // Idle before trigger.
        assert!(!env.is_active());
        assert_eq!(env.render(), 0.0);

        env.trigger();

        // Attack: 10 samples to reach 1.0.
        let mut peak = 0.0_f32;
        for _ in 0..10 {
            peak = env.render();
        }
        assert!(peak >= 0.99, "attack did not reach 1.0 (got {})", peak);

        // Decay: settles at sustain 0.5 within 10 more samples.
        let mut v = peak;
        for _ in 0..10 {
            v = env.render();
        }
        assert!((v - 0.5).abs() < 1e-4, "decay did not land on sustain (got {})", v);

        // Sustain: holds exactly at the sustain level.
        for _ in 0..100 {
            assert_eq!(env.render(), 0.5);
        }

        // Release: decays to exactly 0 and returns to Idle.
        env.release();
        let mut last = 1.0_f32;
        for _ in 0..10 {
            last = env.render();
        }
        assert_eq!(last, 0.0);
        assert!(!env.is_active(), "envelope never returned to idle");
        assert_eq!(env.render(), 0.0);
    }

    #[test]
    fn adsr_release_mid_attack_has_no_amplitude_jump() {
        let mut env = ADSR::new(1000);
        env.set_params(0.1, 0.1, 0.8, 0.05); // slow attack, sustain higher than release point

        env.trigger();
        // Render 20 samples: attack value ~0.2, well below sustain 0.8.
        let mut v = 0.0_f32;
        for _ in 0..20 {
            v = env.render();
        }
        assert!(v < 0.5, "expected to still be early in attack, got {}", v);

        env.release();
        let after = env.render();
        // Must continue downward from the current value, not jump to sustain.
        assert!(
            after <= v + 1e-6,
            "release jumped from {} up to {}",
            v,
            after
        );

        // And it still reaches zero / idle.
        let mut done = false;
        for _ in 0..1000 {
            env.render();
            if !env.is_active() {
                done = true;
                break;
            }
        }
        assert!(done, "release never reached idle");
        assert_eq!(env.get_value(), 0.0);
    }

    #[test]
    fn filter_stable_under_extreme_cutoff_resonance_sweep() {
        let mut noise = Noise(0xDEAD_BEEF);
        let mut filter = LowPassFilter::new(SR);

        // Full-resonance torture: resonance is swept up to and including the
        // maximum a CC can request (1.0), while cutoff sweeps the whole range.
        filter.set_resonance(1.0);
        let mut max_abs = 0.0_f32;
        for i in 0..SR as usize {
            let t = i as f32 / SR as f32;
            let cutoff = 20.0 + t * (18_000.0 - 20.0);
            filter.set_cutoff(cutoff);
            let y = filter.process(noise.next());
            assert!(
                y.is_finite(),
                "filter blew up at sample {} (cutoff {})",
                i,
                cutoff
            );
            max_abs = max_abs.max(y.abs());
        }
        assert!(
            max_abs < 50.0,
            "filter output unbounded at high resonance (max {})",
            max_abs
        );
    }

    #[test]
    fn filter_resonance_one_does_not_nan() {
        // CC 71 at 127 used to set resonance = 1.0, dividing by zero in the
        // coefficient calculation and poisoning the state with NaN forever.
        let mut noise = Noise(42);
        let mut filter = LowPassFilter::new(SR);
        filter.set_cutoff(2000.0);
        filter.set_resonance(1.0);
        for i in 0..10_000 {
            let y = filter.process(noise.next());
            assert!(y.is_finite(), "NaN/inf at sample {}", i);
        }
        // And it must still work after returning to moderate resonance.
        filter.set_resonance(0.2);
        let y = filter.process(0.5);
        assert!(y.is_finite());
    }

    #[test]
    fn filter_attenuates_high_frequencies() {
        let mut filter = LowPassFilter::new(SR);
        filter.set_cutoff(200.0);
        filter.set_resonance(0.0);

        let rms = |filter: &mut LowPassFilter, freq: f32| {
            let mut phase = 0.0_f32;
            let mut acc = 0.0_f32;
            // Skip 2000 transient samples, then measure.
            for i in 0..10_000 {
                phase += freq / SR as f32;
                if phase >= 1.0 {
                    phase -= 1.0;
                }
                let x = (phase * 2.0 * core::f32::consts::PI).sin();
                let y = filter.process(x);
                if i >= 2000 {
                    acc += y * y;
                }
            }
            (acc / 8000.0).sqrt()
        };

        let mut f_low = LowPassFilter::new(SR);
        f_low.set_cutoff(200.0);
        let low = rms(&mut f_low, 50.0);
        let high = rms(&mut filter, 10_000.0);
        assert!(
            high < low * 0.05,
            "low-pass did not attenuate: 50Hz rms={}, 10kHz rms={}",
            low,
            high
        );
    }

    #[test]
    fn delay_impulse_produces_echo_at_tap() {
        let mut delay = Delay::new();
        delay.set_delay_ms(1.0, SR); // 48 samples
        delay.set_feedback(0.0);
        delay.set_mix(1.0); // wet only

        let tap = 48;
        let mut outputs = [0.0_f32; 128];
        for (i, out) in outputs.iter_mut().enumerate() {
            let input = if i == 0 { 1.0 } else { 0.0 };
            *out = delay.process(input);
        }

        // Dry path is fully mixed out; only the echo remains.
        assert_eq!(outputs[0], 0.0);
        for (i, &o) in outputs.iter().enumerate() {
            if i == tap {
                assert!(
                    (o - 1.0).abs() < 1e-6,
                    "expected echo of 1.0 at tap {}, got {}",
                    tap,
                    o
                );
            } else {
                assert_eq!(o, 0.0, "unexpected output at sample {}", i);
            }
        }
    }

    #[test]
    fn delay_feedback_decays_and_stays_bounded() {
        let mut delay = Delay::new();
        delay.set_delay_ms(1.0, SR); // 48 samples
        delay.set_feedback(0.5);
        delay.set_mix(1.0);

        let mut max_abs = 0.0_f32;
        let mut outputs = [0.0_f32; 512];
        for (i, out) in outputs.iter_mut().enumerate() {
            let input = if i == 0 { 1.0 } else { 0.0 };
            *out = delay.process(input);
            assert!(out.is_finite(), "delay output non-finite at {}", i);
            max_abs = max_abs.max(out.abs());
        }

        // Geometric decay: 1.0, 0.5, 0.25, ... at multiples of the tap.
        assert!((outputs[48] - 1.0).abs() < 1e-6);
        assert!((outputs[96] - 0.5).abs() < 1e-6);
        assert!((outputs[144] - 0.25).abs() < 1e-6);
        assert!(max_abs <= 1.0 + 1e-6, "feedback grew unbounded: {}", max_abs);
    }

    #[test]
    fn delay_mix_zero_is_fully_dry() {
        let mut delay = Delay::new();
        delay.set_delay_ms(5.0, SR);
        delay.set_feedback(0.8);
        delay.set_mix(0.0);
        for i in 0..1000 {
            let input = (i as f32 * 0.01).sin();
            let out = delay.process(input);
            assert!(
                (out - input).abs() < 1e-6,
                "mix=0 should be fully dry: in={}, out={}",
                input,
                out
            );
        }
    }
}
