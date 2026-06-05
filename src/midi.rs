#[derive(Debug, Clone, Copy)]
pub enum MidiMessage {
    NoteOn { channel: U4, note: u8, velocity: u8 },
    NoteOff { channel: U4, note: u8, velocity: u8 },
    ControlChange { channel: U4, controller: u8, value: u8 },
    PitchBend { channel: U4, value: i16 },
    ProgramChange { channel: U4, program: u8 },
}

pub struct MidiParser {
    status: u8,
    data: [u8; 2],
    data_index: u8,
}

impl MidiParser {
    pub const fn new() -> Self {
        MidiParser {
            status: 0,
            data: [0; 2],
            data_index: 0,
        }
    }

    pub fn parse_byte(&mut self, byte: u8) -> Option<MidiMessage> {
        if byte >= 0x80 {
            // Status byte
            self.status = byte;
            self.data_index = 0;
            None
        } else {
            // Data byte
            if self.data_index < 2 {
                self.data[self.data_index as usize] = byte;
                self.data_index += 1;
            }

            if self.data_index >= self.data_count() {
                self.data_index = 0;
                self.build_message()
            } else {
                None
            }
        }
    }

    fn data_count(&self) -> u8 {
        match self.status & 0xF0 {
            0x80 | 0x90 | 0xA0 | 0xB0 | 0xE0 => 2,
            0xC0 | 0xD0 => 1,
            _ => 0,
        }
    }

    fn build_message(&self) -> Option<MidiMessage> {
        let channel = self.status & 0x0F;
        match self.status & 0xF0 {
            0x90 if self.data[1] > 0 => Some(MidiMessage::NoteOn {
                channel,
                note: self.data[0],
                velocity: self.data[1],
            }),
            0x80 | 0x90 => Some(MidiMessage::NoteOff {
                channel,
                note: self.data[0],
                velocity: self.data[1],
            }),
            0xB0 => Some(MidiMessage::ControlChange {
                channel,
                controller: self.data[0],
                value: self.data[1],
            }),
            0xE0 => {
                let value = ((self.data[1] as i16) << 7) | (self.data[0] as i16) - 8192;
                Some(MidiMessage::PitchBend { channel, value })
            }
            0xC0 => Some(MidiMessage::ProgramChange {
                channel,
                program: self.data[0],
            }),
            _ => None,
        }
    }
}

type U4 = u8; // MIDI channels are 4-bit
