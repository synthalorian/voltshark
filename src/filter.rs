#[derive(Clone, Copy)]
pub enum FilterType {
    LowPass,
    HighPass,
    BandPass,
}

pub struct Filter {
    cutoff: f32,      // 0.0 - 1.0 (normalized)
    resonance: f32,   // 0.0 - 1.0
    filter_type: FilterType,
    // State variables (Moog ladder approximation)
    z1: f32,
    z2: f32,
    z3: f32,
    z4: f32,
}

impl Filter {
    pub const fn new() -> Self {
        Filter {
            cutoff: 1.0,
            resonance: 0.0,
            filter_type: FilterType::LowPass,
            z1: 0.0,
            z2: 0.0,
            z3: 0.0,
            z4: 0.0,
        }
    }

    pub fn set_cutoff(&mut self, cutoff: f32) {
        self.cutoff = cutoff.clamp(0.0, 1.0);
    }

    pub fn set_resonance(&mut self, resonance: f32) {
        self.resonance = resonance.clamp(0.0, 1.0);
    }

    pub fn process(&mut self, input: f32) -> f32 {
        // Simplified 24dB/octave ladder filter
        let f = self.cutoff * 1.16;
        let fb = self.resonance * 4.0;

        let mut x = input - self.z4 * fb;
        x = x.clamp(-1.0, 1.0); // Saturation

        self.z1 += f * (x - self.z1);
        self.z1 = self.z1.clamp(-1.0, 1.0);

        self.z2 += f * (self.z1 - self.z2);
        self.z2 = self.z2.clamp(-1.0, 1.0);

        self.z3 += f * (self.z2 - self.z3);
        self.z3 = self.z3.clamp(-1.0, 1.0);

        self.z4 += f * (self.z3 - self.z4);
        self.z4 = self.z4.clamp(-1.0, 1.0);

        match self.filter_type {
            FilterType::LowPass => self.z4,
            FilterType::HighPass => input - self.z4,
            FilterType::BandPass => self.z3 - self.z4,
        }
    }
}
