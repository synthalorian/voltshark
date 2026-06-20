use micromath::F32Ext;

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
}

impl Oscillator {
    pub fn new(sample_rate: u32) -> Self {
        Self {
            phase: 0.0,
            phase_increment: 0.0,
            waveform: Waveform::Sawtooth,
            sample_rate: sample_rate as f32,
        }
    }

    pub fn set_frequency(&mut self, freq: f32) {
        self.phase_increment = freq / self.sample_rate;
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
                    self.value = self.sustain * (1.0 - self.phase);
                }
            }
        }

        self.value
    }

    pub fn is_active(&self) -> bool {
        self.state != ADSRState::Idle
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
        self.resonance = resonance.clamp(0.0, 1.0);
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
