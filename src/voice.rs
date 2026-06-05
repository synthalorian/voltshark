use crate::oscillator::Oscillator;
use crate::filter::Filter;
use crate::envelope::Envelope;

pub struct Voice {
    pub active: bool,
    pub note: u8,
    pub velocity: u8,
    pub oscillator: Oscillator,
    pub filter: Filter,
    pub envelope: Envelope,
}

impl Voice {
    pub const fn new() -> Self {
        Voice {
            active: false,
            note: 0,
            velocity: 0,
            oscillator: Oscillator::new(),
            filter: Filter::new(),
            envelope: Envelope::new(),
        }
    }

    pub fn trigger(&mut self, note: u8, velocity: u8) {
        self.active = true;
        self.note = note;
        self.velocity = velocity;

        let freq = midi_note_to_freq(note);
        self.oscillator.set_frequency(freq);
        self.envelope.trigger();
    }

    pub fn release(&mut self) {
        self.envelope.release();
    }

    pub fn next_sample(&mut self) -> i16 {
        let osc = self.oscillator.next_sample();
        let env = self.envelope.next_sample();
        let filtered = self.filter.process(osc * env);

        // Check if envelope finished
        if self.envelope.is_idle() {
            self.active = false;
        }

        (filtered * 32767.0) as i16
    }
}

fn midi_note_to_freq(note: u8) -> f32 {
    440.0 * libm::powf(2.0f32, (note as f32 - 69.0) / 12.0)
}
