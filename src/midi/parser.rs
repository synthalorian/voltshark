/// MIDI event types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MidiEvent {
    NoteOn {
        channel: u8,
        note: u8,
        velocity: u8,
    },
    NoteOff {
        channel: u8,
        note: u8,
        velocity: u8,
    },
    ControlChange {
        channel: u8,
        controller: u8,
        value: u8,
    },
    ProgramChange {
        channel: u8,
        program: u8,
    },
    PitchBend {
        channel: u8,
        value: i16, // -8192 to 8191
    },
    Aftertouch {
        channel: u8,
        note: u8,
        pressure: u8,
    },
    ChannelPressure {
        channel: u8,
        pressure: u8,
    },
    TimingClock,
    Start,
    Continue,
    Stop,
    ActiveSensing,
    Reset,
    SysEx {
        data: [u8; 16],
        len: u8,
    },
}

/// MIDI parser state machine
pub struct MidiParser<U> {
    state: ParseState,
    status: u8,
    data1: u8,
    channel: u8,
    running_status: u8,
    sysex_buffer: [u8; 128],
    sysex_len: usize,
    pub serial: U,
}

#[derive(Debug, Clone, Copy)]
enum ParseState {
    Idle,
    Data1,
    Data2,
    SysEx,
}

impl<U> MidiParser<U> {
    pub fn new(serial: U) -> Self {
        Self {
            state: ParseState::Idle,
            status: 0,
            data1: 0,
            channel: 0,
            running_status: 0,
            sysex_buffer: [0; 128],
            sysex_len: 0,
            serial,
        }
    }

    /// Parse a single MIDI byte
    pub fn parse_byte(&mut self, byte: u8) -> Option<MidiEvent> {
        // System Real-Time messages (can appear anywhere)
        if byte >= 0xF8 {
            return match byte {
                0xF8 => Some(MidiEvent::TimingClock),
                0xFA => Some(MidiEvent::Start),
                0xFB => Some(MidiEvent::Continue),
                0xFC => Some(MidiEvent::Stop),
                0xFE => Some(MidiEvent::ActiveSensing),
                0xFF => Some(MidiEvent::Reset),
                _ => None,
            };
        }

        // Status byte
        if byte & 0x80 != 0 {
            if byte == 0xF0 {
                // Start of SysEx
                self.state = ParseState::SysEx;
                self.sysex_len = 0;
                return None;
            } else if byte == 0xF7 {
                // End of SysEx
                self.state = ParseState::Idle;
                let mut data = [0u8; 16];
                let len = self.sysex_len.min(16);
                data[..len].copy_from_slice(&self.sysex_buffer[..len]);
                return Some(MidiEvent::SysEx {
                    data,
                    len: len as u8,
                });
            } else if byte >= 0xF1 {
                // System Common message (single byte)
                return None;
            }

            // Channel message
            self.status = byte & 0xF0;
            self.channel = byte & 0x0F;
            self.running_status = byte;
            self.state = ParseState::Data1;
            return None;
        }

        // Data byte
        match self.state {
            ParseState::Idle => {
                // Running status - use previous status
                if self.running_status != 0 {
                    self.status = self.running_status & 0xF0;
                    self.channel = self.running_status & 0x0F;
                    self.data1 = byte;
                    self.state = ParseState::Data2;
                    return None;
                }
                None
            }
            ParseState::Data1 => {
                self.data1 = byte;
                match self.status {
                    0xC0 | 0xD0 => {
                        // Program Change or Channel Pressure (1 data byte)
                        self.state = ParseState::Idle;
                        self.parse_message()
                    }
                    _ => {
                        self.state = ParseState::Data2;
                        None
                    }
                }
            }
            ParseState::Data2 => {
                self.state = ParseState::Idle;
                self.parse_message_with_data2(byte)
            }
            ParseState::SysEx => {
                if self.sysex_len < self.sysex_buffer.len() {
                    self.sysex_buffer[self.sysex_len] = byte;
                    self.sysex_len += 1;
                }
                None
            }
        }
    }

    fn parse_message(&self) -> Option<MidiEvent> {
        match self.status {
            0xC0 => Some(MidiEvent::ProgramChange {
                channel: self.channel,
                program: self.data1,
            }),
            0xD0 => Some(MidiEvent::ChannelPressure {
                channel: self.channel,
                pressure: self.data1,
            }),
            _ => None,
        }
    }

    fn parse_message_with_data2(&self, data2: u8) -> Option<MidiEvent> {
        match self.status {
            0x80 => Some(MidiEvent::NoteOff {
                channel: self.channel,
                note: self.data1,
                velocity: data2,
            }),
            0x90 => {
                if data2 == 0 {
                    Some(MidiEvent::NoteOff {
                        channel: self.channel,
                        note: self.data1,
                        velocity: 0,
                    })
                } else {
                    Some(MidiEvent::NoteOn {
                        channel: self.channel,
                        note: self.data1,
                        velocity: data2,
                    })
                }
            }
            0xA0 => Some(MidiEvent::Aftertouch {
                channel: self.channel,
                note: self.data1,
                pressure: data2,
            }),
            0xB0 => Some(MidiEvent::ControlChange {
                channel: self.channel,
                controller: self.data1,
                value: data2,
            }),
            0xE0 => {
                let bend = ((data2 as i16) << 7 | (self.data1 as i16)) - 8192;
                Some(MidiEvent::PitchBend {
                    channel: self.channel,
                    value: bend,
                })
            }
            _ => None,
        }
    }
}

impl<U> MidiParser<U>
where
    U: embedded_hal_nb::serial::Read<u8>,
{
    /// Poll for MIDI events from serial
    pub fn poll(&mut self) -> Option<MidiEvent> {
        match self.serial.read() {
            Ok(byte) => self.parse_byte(byte),
            Err(nb::Error::WouldBlock) => None,
            Err(_) => None,
        }
    }
}

/// MIDI constants
pub mod constants {
    // Control Change numbers
    pub const CC_MODULATION: u8 = 1;
    pub const CC_VOLUME: u8 = 7;
    pub const CC_PAN: u8 = 10;
    pub const CC_RESONANCE: u8 = 71;
    pub const CC_CUTOFF: u8 = 74;
    pub const CC_RESET_ALL_CONTROLLERS: u8 = 121;
    pub const CC_ALL_NOTES_OFF: u8 = 123;
}
