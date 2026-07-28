use crate::synth::dsp::{Delay, LowPassFilter, Oscillator, Waveform, ADSR};
#[cfg_attr(test, allow(unused_imports))] // std shadows these intrinsics in test builds
use micromath::F32Ext; // provides f32 methods (sin, powf, ...) in no_std

/// Patch preset defining a complete synth sound
#[derive(Debug, Clone, Copy)]
pub struct Patch {
    pub name: &'static str,
    pub osc1_waveform: Waveform,
    pub osc2_waveform: Waveform,
    pub osc2_detune: f32,
    pub osc_mix: f32,
    pub filter_cutoff: f32,
    pub filter_resonance: f32,
    pub filter_env_amount: f32,
    pub attack: f32,
    pub decay: f32,
    pub sustain: f32,
    pub release: f32,
    pub filter_attack: f32,
    pub filter_decay: f32,
    pub filter_sustain: f32,
    pub filter_release: f32,
    pub lfo_rate: f32,
    pub delay_ms: f32,
    pub delay_feedback: f32,
    pub delay_mix: f32,
}

impl Patch {
    pub const BASS: Self = Self {
        name: "Bass",
        osc1_waveform: Waveform::Sawtooth,
        osc2_waveform: Waveform::Square,
        osc2_detune: 1.02,
        osc_mix: 0.7,
        filter_cutoff: 400.0,
        filter_resonance: 0.4,
        filter_env_amount: 0.8,
        attack: 0.01,
        decay: 0.2,
        sustain: 0.6,
        release: 0.3,
        filter_attack: 0.01,
        filter_decay: 0.3,
        filter_sustain: 0.2,
        filter_release: 0.3,
        lfo_rate: 0.0,
        delay_ms: 0.0,
        delay_feedback: 0.0,
        delay_mix: 0.0,
    };

    pub const LEAD: Self = Self {
        name: "Lead",
        osc1_waveform: Waveform::Sawtooth,
        osc2_waveform: Waveform::Sawtooth,
        osc2_detune: 1.01,
        osc_mix: 0.5,
        filter_cutoff: 3000.0,
        filter_resonance: 0.3,
        filter_env_amount: 0.5,
        attack: 0.01,
        decay: 0.4,
        sustain: 0.8,
        release: 0.4,
        filter_attack: 0.01,
        filter_decay: 0.5,
        filter_sustain: 0.4,
        filter_release: 0.4,
        lfo_rate: 5.0,
        delay_ms: 120.0,
        delay_feedback: 0.3,
        delay_mix: 0.2,
    };

    pub const PAD: Self = Self {
        name: "Pad",
        osc1_waveform: Waveform::Triangle,
        osc2_waveform: Waveform::Sine,
        osc2_detune: 1.005,
        osc_mix: 0.6,
        filter_cutoff: 800.0,
        filter_resonance: 0.2,
        filter_env_amount: 0.6,
        attack: 0.5,
        decay: 0.8,
        sustain: 0.9,
        release: 1.2,
        filter_attack: 0.5,
        filter_decay: 0.8,
        filter_sustain: 0.5,
        filter_release: 1.0,
        lfo_rate: 2.0,
        delay_ms: 200.0,
        delay_feedback: 0.4,
        delay_mix: 0.3,
    };

    pub const DRUM: Self = Self {
        name: "Drum",
        osc1_waveform: Waveform::Noise,
        osc2_waveform: Waveform::Sine,
        osc2_detune: 1.0,
        osc_mix: 0.9,
        filter_cutoff: 8000.0,
        filter_resonance: 0.1,
        filter_env_amount: 0.9,
        attack: 0.001,
        decay: 0.15,
        sustain: 0.0,
        release: 0.1,
        filter_attack: 0.001,
        filter_decay: 0.2,
        filter_sustain: 0.0,
        filter_release: 0.1,
        lfo_rate: 0.0,
        delay_ms: 50.0,
        delay_feedback: 0.1,
        delay_mix: 0.1,
    };

    pub const PLUCK: Self = Self {
        name: "Pluck",
        osc1_waveform: Waveform::Triangle,
        osc2_waveform: Waveform::Square,
        osc2_detune: 1.03,
        osc_mix: 0.6,
        filter_cutoff: 2000.0,
        filter_resonance: 0.5,
        filter_env_amount: 0.7,
        attack: 0.001,
        decay: 0.3,
        sustain: 0.2,
        release: 0.4,
        filter_attack: 0.001,
        filter_decay: 0.4,
        filter_sustain: 0.1,
        filter_release: 0.3,
        lfo_rate: 0.0,
        delay_ms: 80.0,
        delay_feedback: 0.2,
        delay_mix: 0.15,
    };

    pub const ALL: [Patch; 5] = [Patch::BASS, Patch::LEAD, Patch::PAD, Patch::DRUM, Patch::PLUCK];
}

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
        Self {
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
            pitch_bend: 0.0,
            modulation: 0.0,
            lfo_phase: 0.0,
            lfo_rate: 5.0,
            sample_rate: sr,
        }
    }

    pub fn apply_patch(&mut self, patch: &Patch, _sample_rate: u32) {
        self.osc1.set_waveform(patch.osc1_waveform);
        self.osc2.set_waveform(patch.osc2_waveform);
        self.osc2.set_detune(patch.osc2_detune);
        self.osc_mix = patch.osc_mix;
        self.filter_cutoff_base = patch.filter_cutoff;
        self.filter_resonance = patch.filter_resonance;
        self.filter_env_amount = patch.filter_env_amount;
        self.lfo_rate = patch.lfo_rate;
        self.envelope.set_params(patch.attack, patch.decay, patch.sustain, patch.release);
        self.filter_envelope.set_params(patch.filter_attack, patch.filter_decay, patch.filter_sustain, patch.filter_release);
    }

    pub fn trigger(&mut self, note: u8, velocity: u8) {
        self.note = note;
        self.velocity = velocity;
        self.active = true;
        self.pitch_bend = 0.0;

        let freq = Self::note_to_frequency(note);
        self.osc1.set_frequency(freq);
        self.osc2.set_frequency(freq * self.osc2.get_detune());

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
        self.osc2.set_frequency(freq * self.osc2.get_detune());

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

        // Check if voice is done
        if !self.envelope.is_active() && env_value <= 0.001 {
            self.active = false;
        }

        output
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

/// Polyphonic synthesizer engine with patch support
pub struct SynthEngine {
    voices: [Voice; 16],
    sample_rate: u32,
    master_volume: f32,
    pan_spread: f32,
    current_patch_index: usize,

    // Global delay send effect (stereo). One shared effect rather than a
    // per-voice buffer: 16 per-voice delay lines would need >512KB of RAM,
    // far beyond the STM32F4's 192KB SRAM.
    delay_left: Delay,
    delay_right: Delay,

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
            current_patch_index: 0,
            delay_left: Delay::new(),
            delay_right: Delay::new(),
            cutoff: 2000.0,
            resonance: 0.3,
            attack: 0.01,
            decay: 0.3,
            sustain: 0.7,
            release: 0.5,
        };

        // Apply default patch to all voices
        let patch = Patch::ALL[0];
        for voice in &mut engine.voices {
            voice.apply_patch(&patch, sample_rate);
        }
        engine.apply_patch_delay(&patch);

        engine
    }

    fn apply_patch_delay(&mut self, patch: &Patch) {
        self.delay_left.set_delay_ms(patch.delay_ms, self.sample_rate);
        self.delay_right.set_delay_ms(patch.delay_ms, self.sample_rate);
        self.delay_left.set_feedback(patch.delay_feedback);
        self.delay_right.set_feedback(patch.delay_feedback);
        self.delay_left.set_mix(patch.delay_mix);
        self.delay_right.set_mix(patch.delay_mix);
    }

    pub fn set_patch(&mut self, index: usize) {
        let index = index % Patch::ALL.len();
        self.current_patch_index = index;
        let patch = Patch::ALL[index];
        for voice in &mut self.voices {
            voice.apply_patch(&patch, self.sample_rate);
        }
        self.apply_patch_delay(&patch);
    }

    pub fn next_patch(&mut self) {
        self.set_patch(self.current_patch_index + 1);
    }

    pub fn get_patch_name(&self) -> &'static str {
        Patch::ALL[self.current_patch_index].name
    }

    pub fn note_on(&mut self, _channel: u8, note: u8, velocity: u8) {
        // Find free voice (steal quietest if none free)
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
            1 => {
                // Modulation
                for voice in &mut self.voices {
                    voice.modulation = normalized;
                }
            }
            7 => {
                // Volume
                self.master_volume = normalized;
            }
            10 => {
                // Pan
                self.pan_spread = normalized * 0.5;
            }
            71 => {
                // Resonance
                self.resonance = normalized;
                for voice in &mut self.voices {
                    voice.filter_resonance = self.resonance;
                }
            }
            74 => {
                // Cutoff
                self.cutoff = 100.0 + normalized * 8000.0;
                for voice in &mut self.voices {
                    voice.filter_cutoff_base = self.cutoff;
                }
            }
            121 => {
                // Reset all controllers
                self.reset_controllers();
            }
            123 => {
                // All notes off
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

        // Global stereo delay send effect
        left = self.delay_left.process(left);
        right = self.delay_right.process(right);

        (left, right)
    }

    fn find_free_voice(&self) -> usize {
        // First, try to find an inactive voice
        for (i, voice) in self.voices.iter().enumerate() {
            if !voice.active && !voice.envelope.is_active() {
                return i;
            }
        }

        // Steal the voice with the lowest envelope value (quietest = best to steal)
        let mut min_env = f32::MAX;
        let mut min_index = 0;
        for (i, voice) in self.voices.iter().enumerate() {
            let env = voice.envelope.get_value();
            if env < min_env {
                min_env = env;
                min_index = i;
            }
        }

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

#[cfg(test)]
mod tests {
    use super::*;

    const SR: u32 = 48_000;

    fn engine() -> SynthEngine {
        SynthEngine::new(SR)
    }

    #[test]
    fn note_on_assigns_free_voice() {
        let mut e = engine();
        e.note_on(0, 60, 100);
        assert!(e.voices[0].active);
        assert_eq!(e.voices[0].note, 60);
        assert_eq!(e.voices[0].velocity, 100);
        // All other voices untouched.
        assert!(e.voices[1..].iter().all(|v| !v.active));
    }

    #[test]
    fn voice_stealing_at_16_voice_limit() {
        let mut e = engine();
        for i in 0..16u8 {
            e.note_on(0, 40 + i, 100);
        }
        assert!(e.voices.iter().all(|v| v.active));

        // 17th note must steal a voice (all envelopes still at 0, so the
        // quietest-voice search steals the first one).
        e.note_on(0, 90, 100);
        assert_eq!(
            e.voices.iter().filter(|v| v.active).count(),
            16,
            "more than 16 voices active after stealing"
        );
        assert!(
            e.voices.iter().any(|v| v.note == 90),
            "new note not assigned after steal"
        );
        assert!(
            !e.voices.iter().any(|v| v.note == 40),
            "stolen voice still holds its old note"
        );
    }

    #[test]
    fn note_off_releases_the_right_voice() {
        let mut e = engine();
        e.note_on(0, 60, 100);
        e.note_on(0, 67, 100);
        e.note_off(0, 60);

        // Default patch release is 0.3s; render well past that.
        for _ in 0..SR as usize {
            e.render();
        }

        let released = e.voices.iter().find(|v| v.note == 60).unwrap();
        let held = e.voices.iter().find(|v| v.note == 67).unwrap();
        assert!(!released.active, "released voice never went inactive");
        assert!(held.active, "held voice was wrongly released");
    }

    #[test]
    fn per_voice_adsr_independence() {
        let mut e = engine();
        e.note_on(0, 60, 100);
        e.note_on(0, 64, 100);
        // Let both envelopes get into their cycle.
        for _ in 0..2000 {
            e.render();
        }
        e.note_off(0, 60);

        // Shortly after the note-off, the released voice's envelope must be
        // falling while the held voice keeps sounding.
        let mut released_peak = 0.0_f32;
        let mut held_sum = 0.0_f32;
        for _ in 0..2000 {
            e.render();
            let rel = e.voices.iter().find(|v| v.note == 60).unwrap();
            let held = e.voices.iter().find(|v| v.note == 64).unwrap();
            released_peak = released_peak.max(rel.envelope.get_value());
            held_sum += held.envelope.get_value();
        }
        let rel = e.voices.iter().find(|v| v.note == 60).unwrap();
        let held = e.voices.iter().find(|v| v.note == 64).unwrap();
        assert!(
            rel.envelope.get_value() < held.envelope.get_value(),
            "released envelope ({}) not below held envelope ({})",
            rel.envelope.get_value(),
            held.envelope.get_value()
        );
        assert!(held_sum > 0.0, "held voice envelope silent");
        let _ = released_peak;
    }

    #[test]
    fn cc74_changes_cutoff() {
        let mut e = engine();
        e.control_change(0, 74, 127);
        assert!((e.cutoff - 8100.0).abs() < 1e-3);
        assert!(e.voices.iter().all(|v| (v.filter_cutoff_base - 8100.0).abs() < 1e-3));

        e.control_change(0, 74, 0);
        assert!((e.cutoff - 100.0).abs() < 1e-3);
    }

    #[test]
    fn cc71_changes_resonance() {
        let mut e = engine();
        e.control_change(0, 71, 64);
        let expected = 64.0 / 127.0;
        assert!((e.resonance - expected).abs() < 1e-6);
        assert!(e
            .voices
            .iter()
            .all(|v| (v.filter_resonance - expected).abs() < 1e-6));
    }

    #[test]
    fn cc71_max_resonance_does_not_blow_up_render() {
        // Regression: CC 71 = 127 used to drive filter resonance to exactly
        // 1.0, producing NaN coefficients and killing all audio permanently.
        let mut e = engine();
        e.control_change(0, 71, 127);
        e.note_on(0, 60, 127);
        for i in 0..10_000 {
            let (l, r) = e.render();
            assert!(l.is_finite() && r.is_finite(), "non-finite render at {}", i);
        }
    }

    #[test]
    fn cc7_controls_master_volume() {
        let mut e = engine();
        e.control_change(0, 7, 0);
        e.note_on(0, 60, 127);
        let mut peak = 0.0_f32;
        for _ in 0..4000 {
            let (l, r) = e.render();
            peak = peak.max(l.abs()).max(r.abs());
        }
        assert_eq!(peak, 0.0, "volume 0 should silence output, got {}", peak);
    }

    #[test]
    fn cc1_sets_modulation_and_cc10_sets_pan() {
        let mut e = engine();
        e.control_change(0, 1, 127);
        assert!(e.voices.iter().all(|v| (v.modulation - 1.0).abs() < 1e-6));
        e.control_change(0, 10, 127);
        assert!((e.pan_spread - 0.5).abs() < 1e-6);
    }

    #[test]
    fn pitch_bend_range_maps_to_plus_minus_one() {
        let mut e = engine();
        e.pitch_bend(0, -8192);
        assert!(e.voices.iter().all(|v| (v.pitch_bend - -1.0).abs() < 1e-6));
        e.pitch_bend(0, 8191);
        assert!(e
            .voices
            .iter()
            .all(|v| (v.pitch_bend - 8191.0 / 8192.0).abs() < 1e-6));
        e.pitch_bend(0, 0);
        assert!(e.voices.iter().all(|v| v.pitch_bend == 0.0));
    }

    #[test]
    fn pitch_bend_actually_shifts_rendered_frequency() {
        // Bend fully up (+2 semitones): the rendered waveform must complete
        // more cycles than the unbent one over the same window.
        let cycles = |bend: i16| {
            let mut e = engine();
            e.set_patch(0); // Bass, no LFO
            e.note_on(0, 69, 127); // A4 = 440 Hz
            e.pitch_bend(0, bend);
            let mut crossings = 0u32;
            let mut prev = 0.0_f32;
            for _ in 0..SR as usize / 10 {
                let (l, _) = e.render();
                if prev <= 0.0 && l > 0.0 {
                    crossings += 1;
                }
                prev = l;
            }
            crossings
        };
        let flat = cycles(0);
        let up = cycles(8191);
        // +2 semitones = ratio 2^(2/12) ~= 1.1225 -> expect ~12% more cycles.
        assert!(
            up > flat + flat / 10,
            "bend up did not raise pitch: flat={}, up={}",
            flat,
            up
        );
    }

    #[test]
    fn dual_oscillator_detune_produces_chorus() {
        // Patches detune osc2 (e.g. Bass = 1.02). A single-oscillator render
        // would be periodic; the detuned pair must beat/diverge. Compare
        // against an engine whose osc2 detune we force back to unison.
        let render_energy_diff = |detune2: f32| {
            let mut e = engine();
            e.set_patch(0);
            for v in &mut e.voices {
                v.osc2.set_detune(detune2);
            }
            e.note_on(0, 50, 127);
            let mut out = [0.0_f32; 4096];
            for (i, s) in out.iter_mut().enumerate() {
                let _ = i;
                let (l, _) = e.render();
                *s = l;
            }
            out
        };
        let detuned = render_energy_diff(1.02);
        let unison = render_energy_diff(1.0);
        let diff: f32 = detuned
            .iter()
            .zip(unison.iter())
            .map(|(a, b)| (a - b).abs())
            .sum();
        assert!(
            diff > 10.0,
            "detuned osc2 made no audible difference (sum diff {})",
            diff
        );
    }

    #[test]
    fn patch_switching() {
        let mut e = engine();
        assert_eq!(e.get_patch_name(), "Bass");
        e.set_patch(3);
        assert_eq!(e.get_patch_name(), "Drum");
        e.next_patch();
        assert_eq!(e.get_patch_name(), "Pluck");
        e.set_patch(99); // wraps
        assert_eq!(e.get_patch_name(), "Pluck");
    }

    #[test]
    fn all_notes_off_and_reset_controllers() {
        let mut e = engine();
        e.note_on(0, 60, 100);
        e.note_on(0, 64, 100);
        e.all_notes_off();
        for _ in 0..SR as usize {
            e.render();
        }
        assert!(e.voices.iter().all(|v| !v.active));

        e.control_change(0, 74, 127);
        e.control_change(0, 71, 100);
        e.reset_controllers();
        assert!((e.cutoff - 2000.0).abs() < 1e-3);
        assert!((e.resonance - 0.3).abs() < 1e-6);
    }

    #[test]
    fn render_output_is_finite_and_bounded() {
        let mut e = engine();
        // Slam all 16 voices at max velocity.
        for i in 0..16u8 {
            e.note_on(0, 48 + i, 127);
        }
        for i in 0..10_000 {
            let (l, r) = e.render();
            assert!(l.is_finite() && r.is_finite(), "non-finite sample at {}", i);
            assert!(
                l.abs() <= 1.0 && r.abs() <= 1.0,
                "soft limiter exceeded: ({}, {})",
                l,
                r
            );
        }
    }
}
