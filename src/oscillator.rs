#[derive(Clone, Copy)]
pub enum Waveform {
    Sine,
    Triangle,
    Saw,
    Square,
    Pulse { width: f32 },
}

pub struct Oscillator {
    phase: f32,
    phase_increment: f32,
    waveform: Waveform,
    sample_rate: f32,
}

impl Oscillator {
    pub const fn new() -> Self {
        Oscillator {
            phase: 0.0,
            phase_increment: 0.0,
            waveform: Waveform::Saw,
            sample_rate: 48000.0,
        }
    }

    pub fn set_frequency(&mut self, freq: f32) {
        self.phase_increment = freq / self.sample_rate;
    }

    pub fn set_waveform(&mut self, waveform: Waveform) {
        self.waveform = waveform;
    }

    pub fn next_sample(&mut self) -> f32 {
        self.phase += self.phase_increment;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }

        match self.waveform {
            Waveform::Sine => libm::sinf(self.phase * 2.0 * core::f32::consts::PI),
            Waveform::Triangle => {
                if self.phase < 0.5 {
                    4.0 * self.phase - 1.0
                } else {
                    3.0 - 4.0 * self.phase
                }
            }
            Waveform::Saw => 2.0 * self.phase - 1.0,
            Waveform::Square => {
                if self.phase < 0.5 { 1.0 } else { -1.0 }
            }
            Waveform::Pulse { width } => {
                if self.phase < width { 1.0 } else { -1.0 }
            }
        }
    }
}
