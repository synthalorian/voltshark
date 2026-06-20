use crate::synth::dsp::{ADSRState, Delay, LowPassFilter, Oscillator, Waveform, ADSR};
use micromath::F32Ext;

/// Polyphonic voice with full synth architecture
pub struct Voice {
    pub note: u8,
    pub velocity: u8,
    pub active: bool,

    // Oscillators
    pub osc1: Oscillator,
    pub osc2: Oscillator,
    pub osc_mix: f32,

    // Envelope
    pub envelope: ADSR,
    pub filter_envelope: ADSR,

    // Filter
    pub filter: LowPassFilter,
    pub filter_cutoff_base: f32,
    pub filter_resonance: f32,
    pub filter_env_amount: f32,

    // Effects
    pub delay: Delay,

    // Modulation
    pub pitch_bend: f32,
    pub modulation: f32,
    pub lfo_phase: f32,
    pub lfo_rate: f32,

    sample_rate: f32,
}

impl Voice {
    pub fn new(sample_rate: u32) -> Self {
        let sr = sample_rate as f32;
        let mut voice = Self {
            note: 0,
            velocity: 0,
            active: false,
            osc1: Oscillator::new(sample_rate),
            osc2: Oscillator::new(sample_rate),
            osc_mix: 0.5,
            envelope: ADSR::new(sample_rate),
            filter_envelope: ADSR::new(sample_rate),
            filter: LowPassFilter::new(sample_rate),
            filter_cutoff_base: 2000.0,
            filter_resonance: 0.3,
            filter_env_amount: 0.5,
            delay: Delay::new(),
            pitch_bend: 0.0,
            modulation: 0.0,
            lfo_phase: 0.0,
            lfo_rate: 5.0,
            sample_rate: sr,
        };

        voice.osc1.set_waveform(Waveform::Sawtooth);
        voice.osc2.set_waveform(Waveform::Square);
        voice.osc2.set_frequency(0.0); // Will be set on note on
        voice.envelope.set_params(0.01, 0.3, 0.7, 0.5);
        voice.filter_envelope.set_params(0.01, 0.5, 0.0, 0.3);
        voice.delay.set_delay_ms(100.0, sample_rate);

        voice
    }

    pub fn trigger(&mut self, note: u8, velocity: u8) {
        self.note = note;
        self.velocity = velocity;
        self.active = true;
        self.pitch_bend = 0.0;

        let freq = Self::note_to_frequency(note);
        self.osc1.set_frequency(freq);
        self.osc2.set_frequency(freq * 1.01); // Slight detune for thickness

        self.envelope.trigger();
        self.filter_envelope.trigger();
        self.filter.reset();
    }

    pub fn release(&mut self) {
        self.envelope.release();
        self.filter_envelope.release();
    }

    pub fn render(&mut self) -> f32 {
        if !self.active && !self.envelope.is_active() {
            return 0.0;
        }

        // Calculate pitch with bend
        let bend_semitones = self.pitch_bend * 2.0; // +/- 2 semitones
        let bend_ratio = Self::semitones_to_ratio(bend_semitones);
        let base_freq = Self::note_to_frequency(self.note);
        let freq = base_freq * bend_ratio;

        self.osc1.set_frequency(freq);
        self.osc2.set_frequency(freq * 1.01);

        // LFO for vibrato
        self.lfo_phase += self.lfo_rate / self.sample_rate;
        if self.lfo_phase >= 1.0 {
            self.lfo_phase -= 1.0;
        }
        let lfo = (self.lfo_phase * 2.0 * core::f32::consts::PI).sin() * self.modulation * 0.1;
        self.osc1.set_frequency(freq * (1.0 + lfo));

        // Render oscillators
        let osc1_sample = self.osc1.render();
        let osc2_sample = self.osc2.render();
        let mixed = osc1_sample * self.osc_mix + osc2_sample * (1.0 - self.osc_mix);

        // Apply envelope
        let env_value = self.envelope.render();
        let filtered = mixed * env_value;

        // Filter with envelope modulation
        let filter_env = self.filter_envelope.render();
        let cutoff = self.filter_cutoff_base + filter_env * self.filter_env_amount * 8000.0;
        self.filter.set_cutoff(cutoff.clamp(20.0, 18000.0));
        self.filter.set_resonance(self.filter_resonance);
        let filtered_sample = self.filter.process(filtered);

        // Apply velocity scaling
        let velocity_scale = (self.velocity as f32) / 127.0;
        let output = filtered_sample * velocity_scale * 0.5;

        // Apply delay effect
        let delayed = self.delay.process(output);

        // Check if voice is done
        if !self.envelope.is_active() && env_value <= 0.001 {
            self.active = false;
        }

        delayed
    }

    fn note_to_frequency(note: u8) -> f32 {
        // A4 = 69 = 440Hz
        let n = note as f32 - 69.0;
        440.0 * (2.0_f32).powf(n / 12.0)
    }

    fn semitones_to_ratio(semitones: f32) -> f32 {
        (2.0_f32).powf(semitones / 12.0)
    }
}

/// Polyphonic synthesizer engine
pub struct SynthEngine {
    voices: [Voice; 16],
    sample_rate: u32,
    master_volume: f32,
    pan_spread: f32,

    // Global parameters
    pub cutoff: f32,
    pub resonance: f32,
    pub attack: f32,
    pub decay: f32,
    pub sustain: f32,
    pub release: f32,
}

impl SynthEngine {
    pub fn new(sample_rate: u32) -> Self {
        let mut engine = Self {
            voices: core::array::from_fn(|_| Voice::new(sample_rate)),
            sample_rate,
            master_volume: 0.8,
            pan_spread: 0.3,
            cutoff: 2000.0,
            resonance: 0.3,
            attack: 0.01,
            decay: 0.3,
            sustain: 0.7,
            release: 0.5,
        };

        // Set initial parameters on all voices
        for voice in &mut engine.voices {
            voice.filter_cutoff_base = engine.cutoff;
            voice.filter_resonance = engine.resonance;
            voice
                .envelope
                .set_params(engine.attack, engine.decay, engine.sustain, engine.release);
        }

        engine
    }

    pub fn note_on(&mut self, _channel: u8, note: u8, velocity: u8) {
        // Find free voice (steal oldest if none free)
        let voice_index = self.find_free_voice();
        self.voices[voice_index].trigger(note, velocity);
    }

    pub fn note_off(&mut self, _channel: u8, note: u8) {
        for voice in &mut self.voices {
            if voice.active && voice.note == note {
                voice.release();
            }
        }
    }

    pub fn control_change(&mut self, _channel: u8, controller: u8, value: u8) {
        let normalized = (value as f32) / 127.0;

        match controller {
            1 => { // Modulation
                for voice in &mut self.voices {
                    voice.modulation = normalized;
                }
            }
            7 => { // Volume
                self.master_volume = normalized;
            }
            10 => { // Pan
                self.pan_spread = normalized * 0.5;
            }
            71 => { // Resonance
                self.resonance = normalized;
                for voice in &mut self.voices {
                    voice.filter_resonance = self.resonance;
                }
            }
            74 => { // Cutoff
                self.cutoff = 100.0 + normalized * 8000.0;
                for voice in &mut self.voices {
                    voice.filter_cutoff_base = self.cutoff;
                }
            }
            121 => { // Reset all controllers
                self.reset_controllers();
            }
            123 => { // All notes off
                self.all_notes_off();
            }
            _ => {}
        }
    }

    pub fn pitch_bend(&mut self, _channel: u8, value: i16) {
        let normalized = value as f32 / 8192.0;
        for voice in &mut self.voices {
            voice.pitch_bend = normalized;
        }
    }

    pub fn all_notes_off(&mut self) {
        for voice in &mut self.voices {
            voice.release();
        }
    }

    pub fn reset_controllers(&mut self) {
        self.cutoff = 2000.0;
        self.resonance = 0.3;
        self.attack = 0.01;
        self.decay = 0.3;
        self.sustain = 0.7;
        self.release = 0.5;
        self.master_volume = 0.8;
        self.pan_spread = 0.3;

        for voice in &mut self.voices {
            voice.filter_cutoff_base = self.cutoff;
            voice.filter_resonance = self.resonance;
            voice
                .envelope
                .set_params(self.attack, self.decay, self.sustain, self.release);
            voice.modulation = 0.0;
            voice.pitch_bend = 0.0;
        }
    }

    pub fn render(&mut self) -> (f32, f32) {
        let mut left = 0.0_f32;
        let mut right = 0.0_f32;
        let mut active_voices = 0;

        for (i, voice) in self.voices.iter_mut().enumerate() {
            if voice.active || voice.envelope.is_active() {
                let sample = voice.render();
                active_voices += 1;

                // Simple panning based on voice index
                let pan = ((i as f32) / 16.0 - 0.5) * self.pan_spread * 2.0;
                let left_gain = (1.0 - pan).clamp(0.0, 1.0);
                let right_gain = (1.0 + pan).clamp(0.0, 1.0);

                left += sample * left_gain;
                right += sample * right_gain;
            }
        }

        // Soft limiting to prevent clipping
        let master = self.master_volume / (1.0 + active_voices as f32 * 0.1);
        left = Self::soft_limit(left * master);
        right = Self::soft_limit(right * master);

        (left, right)
    }

    fn find_free_voice(&self) -> usize {
        // First, try to find an inactive voice
        for (i, voice) in self.voices.iter().enumerate() {
            if !voice.active && !voice.envelope.is_active() {
                return i;
            }
        }

        // Steal the voice with the lowest envelope value
        let mut min_env = f32::MAX;
        let mut min_index = 0;
        for (i, voice) in self.voices.iter().enumerate() {
            // Approximate by checking if in release stage
            if !voice.active {
                return i;
            }
        }

        // Fallback: steal voice 0
        min_index
    }

    fn soft_limit(sample: f32) -> f32 {
        // Soft clipping: tanh-like curve
        if sample > 1.0 {
            1.0 - (1.0 / (sample + 1.0))
        } else if sample < -1.0 {
            -1.0 + (1.0 / (-sample + 1.0))
        } else {
            sample
        }
    }
}
