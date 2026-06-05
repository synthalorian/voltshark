#[derive(Clone, Copy)]
pub struct ADSR {
    pub attack: f32,   // seconds
    pub decay: f32,    // seconds
    pub sustain: f32,  // level 0.0-1.0
    pub release: f32,  // seconds
}

impl ADSR {
    pub const fn default() -> Self {
        ADSR {
            attack: 0.01,
            decay: 0.1,
            sustain: 0.7,
            release: 0.3,
        }
    }
}

#[derive(Clone, Copy)]
pub enum EnvelopeState {
    Idle,
    Attack,
    Decay,
    Sustain,
    Release,
}

pub struct Envelope {
    adsr: ADSR,
    state: EnvelopeState,
    level: f32,
    sample_rate: f32,
}

impl Envelope {
    pub const fn new() -> Self {
        Envelope {
            adsr: ADSR::default(),
            state: EnvelopeState::Idle,
            level: 0.0,
            sample_rate: 48000.0,
        }
    }

    pub fn trigger(&mut self) {
        self.state = EnvelopeState::Attack;
    }

    pub fn release(&mut self) {
        self.state = EnvelopeState::Release;
    }

    pub fn is_idle(&self) -> bool {
        matches!(self.state, EnvelopeState::Idle)
    }

    pub fn next_sample(&mut self) -> f32 {
        match self.state {
            EnvelopeState::Idle => 0.0,
            EnvelopeState::Attack => {
                let increment = 1.0 / (self.adsr.attack * self.sample_rate);
                self.level += increment;
                if self.level >= 1.0 {
                    self.level = 1.0;
                    self.state = EnvelopeState::Decay;
                }
                self.level
            }
            EnvelopeState::Decay => {
                let increment = (1.0 - self.adsr.sustain) / (self.adsr.decay * self.sample_rate);
                self.level -= increment;
                if self.level <= self.adsr.sustain {
                    self.level = self.adsr.sustain;
                    self.state = EnvelopeState::Sustain;
                }
                self.level
            }
            EnvelopeState::Sustain => self.adsr.sustain,
            EnvelopeState::Release => {
                let increment = self.adsr.sustain / (self.adsr.release * self.sample_rate);
                self.level -= increment;
                if self.level <= 0.0 {
                    self.level = 0.0;
                    self.state = EnvelopeState::Idle;
                }
                self.level
            }
        }
    }
}
