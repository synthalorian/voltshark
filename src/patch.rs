use crate::oscillator::Waveform;
use crate::filter::FilterType;
use crate::envelope::ADSR;

#[derive(Clone, Copy)]
pub struct Patch {
    pub name: [u8; 16],
    pub oscillator_waveform: Waveform,
    pub filter_type: FilterType,
    pub filter_cutoff: f32,
    pub filter_resonance: f32,
    pub envelope: ADSR,
    pub volume: f32,
}

impl Patch {
    pub const fn init() -> Self {
        Patch {
            name: *b"Init Patch      ",
            oscillator_waveform: Waveform::Saw,
            filter_type: FilterType::LowPass,
            filter_cutoff: 1.0,
            filter_resonance: 0.0,
            envelope: ADSR::default(),
            volume: 0.8,
        }
    }

    pub fn to_bytes(&self) -> [u8; 64] {
        // TODO: Serialize patch to bytes for storage
        [0; 64]
    }

    pub fn from_bytes(_bytes: &[u8]) -> Self {
        // TODO: Deserialize patch from bytes
        Patch::init()
    }
}
