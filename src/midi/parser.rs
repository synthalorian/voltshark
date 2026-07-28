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
                    // Program Change and Channel Pressure carry a single data
                    // byte; emit immediately instead of waiting for a second.
                    if matches!(self.status, 0xC0 | 0xD0) {
                        return self.parse_message();
                    }
                    self.state = ParseState::Data2;
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Parser with a dummy serial port; tests drive parse_byte directly.
    fn parser() -> MidiParser<()> {
        MidiParser::new(())
    }

    fn feed(p: &mut MidiParser<()>, bytes: &[u8]) -> Option<MidiEvent> {
        let mut event = None;
        for &b in bytes {
            if let Some(e) = p.parse_byte(b) {
                event = Some(e);
            }
        }
        event
    }

    #[test]
    fn note_on_basic() {
        let mut p = parser();
        assert_eq!(p.parse_byte(0x90), None);
        assert_eq!(p.parse_byte(60), None);
        assert_eq!(
            p.parse_byte(100),
            Some(MidiEvent::NoteOn {
                channel: 0,
                note: 60,
                velocity: 100
            })
        );
    }

    #[test]
    fn note_off_basic() {
        let mut p = parser();
        let e = feed(&mut p, &[0x83, 60, 64]);
        assert_eq!(
            e,
            Some(MidiEvent::NoteOff {
                channel: 3,
                note: 60,
                velocity: 64
            })
        );
    }

    #[test]
    fn note_on_velocity_zero_is_note_off() {
        let mut p = parser();
        let e = feed(&mut p, &[0x90, 60, 0]);
        assert_eq!(
            e,
            Some(MidiEvent::NoteOff {
                channel: 0,
                note: 60,
                velocity: 0
            })
        );
    }

    #[test]
    fn running_status_note_messages() {
        let mut p = parser();
        // One status byte, then three messages worth of data.
        assert_eq!(feed(&mut p, &[0x90, 60, 100]), Some(MidiEvent::NoteOn { channel: 0, note: 60, velocity: 100 }));
        assert_eq!(feed(&mut p, &[62, 100]), Some(MidiEvent::NoteOn { channel: 0, note: 62, velocity: 100 }));
        assert_eq!(feed(&mut p, &[64, 0]), Some(MidiEvent::NoteOff { channel: 0, note: 64, velocity: 0 }));
    }

    #[test]
    fn running_status_program_change_single_data_byte() {
        let mut p = parser();
        assert_eq!(
            feed(&mut p, &[0xC0, 5]),
            Some(MidiEvent::ProgramChange {
                channel: 0,
                program: 5
            })
        );
        // Running status: next data byte alone is another program change,
        // not the first byte of a two-byte message.
        assert_eq!(
            p.parse_byte(7),
            Some(MidiEvent::ProgramChange {
                channel: 0,
                program: 7
            })
        );
    }

    #[test]
    fn control_change_parsing() {
        let mut p = parser();
        let e = feed(&mut p, &[0xB2, 74, 127]);
        assert_eq!(
            e,
            Some(MidiEvent::ControlChange {
                channel: 2,
                controller: 74,
                value: 127
            })
        );
    }

    #[test]
    fn pitch_bend_14bit_decode() {
        let mut p = parser();
        // Center: LSB 0x00, MSB 0x40 -> 0
        assert_eq!(
            feed(&mut p, &[0xE0, 0x00, 0x40]),
            Some(MidiEvent::PitchBend {
                channel: 0,
                value: 0
            })
        );
        // Max: 0x3FFF - 0x2000 = 8191
        assert_eq!(
            feed(&mut p, &[0xE0, 0x7F, 0x7F]),
            Some(MidiEvent::PitchBend {
                channel: 0,
                value: 8191
            })
        );
        // Min: 0 - 8192 = -8192
        assert_eq!(
            feed(&mut p, &[0xE0, 0x00, 0x00]),
            Some(MidiEvent::PitchBend {
                channel: 0,
                value: -8192
            })
        );
    }

    #[test]
    fn channel_pressure_parsing() {
        let mut p = parser();
        let e = feed(&mut p, &[0xD5, 99]);
        assert_eq!(
            e,
            Some(MidiEvent::ChannelPressure {
                channel: 5,
                pressure: 99
            })
        );
    }

    #[test]
    fn polyphonic_aftertouch_parsing() {
        let mut p = parser();
        let e = feed(&mut p, &[0xA1, 60, 50]);
        assert_eq!(
            e,
            Some(MidiEvent::Aftertouch {
                channel: 1,
                note: 60,
                pressure: 50
            })
        );
    }

    #[test]
    fn stray_data_byte_without_status_produces_nothing() {
        let mut p = parser();
        assert_eq!(p.parse_byte(60), None);
        assert_eq!(p.parse_byte(100), None);
        assert_eq!(p.parse_byte(0x40), None);
    }

    #[test]
    fn truncated_message_emits_nothing_until_complete() {
        let mut p = parser();
        assert_eq!(p.parse_byte(0x90), None);
        assert_eq!(p.parse_byte(60), None); // only one data byte so far
        // Complete it
        assert!(matches!(p.parse_byte(100), Some(MidiEvent::NoteOn { .. })));
    }

    #[test]
    fn new_status_aborts_partial_message() {
        let mut p = parser();
        assert_eq!(p.parse_byte(0x90), None);
        assert_eq!(p.parse_byte(60), None); // partial note on
        // A new status byte replaces the pending message.
        assert_eq!(p.parse_byte(0xB0), None);
        assert_eq!(p.parse_byte(7), None);
        assert_eq!(
            p.parse_byte(127),
            Some(MidiEvent::ControlChange {
                channel: 0,
                controller: 7,
                value: 127
            })
        );
    }

    #[test]
    fn realtime_messages_interleave_without_disturbing_state() {
        let mut p = parser();
        assert_eq!(p.parse_byte(0x90), None);
        assert_eq!(p.parse_byte(60), None);
        // Timing clock arrives mid-message.
        assert_eq!(p.parse_byte(0xF8), Some(MidiEvent::TimingClock));
        // The pending note-on completes normally.
        assert_eq!(
            p.parse_byte(100),
            Some(MidiEvent::NoteOn {
                channel: 0,
                note: 60,
                velocity: 100
            })
        );
    }

    #[test]
    fn realtime_start_stop_continue_sensing_reset() {
        let mut p = parser();
        assert_eq!(p.parse_byte(0xFA), Some(MidiEvent::Start));
        assert_eq!(p.parse_byte(0xFB), Some(MidiEvent::Continue));
        assert_eq!(p.parse_byte(0xFC), Some(MidiEvent::Stop));
        assert_eq!(p.parse_byte(0xFE), Some(MidiEvent::ActiveSensing));
        assert_eq!(p.parse_byte(0xFF), Some(MidiEvent::Reset));
    }

    #[test]
    fn sysex_collect_and_terminate() {
        let mut p = parser();
        let e = feed(&mut p, &[0xF0, 0x01, 0x02, 0x03, 0xF7]);
        match e {
            Some(MidiEvent::SysEx { data, len }) => {
                assert_eq!(len, 3);
                assert_eq!(&data[..3], &[0x01, 0x02, 0x03]);
            }
            other => panic!("expected SysEx, got {:?}", other),
        }
    }

    #[test]
    fn channel_field_is_preserved_not_filtered() {
        // The parser performs no channel filtering; channel is passed through.
        let mut p = parser();
        let e = feed(&mut p, &[0x9F, 60, 100]);
        assert_eq!(
            e,
            Some(MidiEvent::NoteOn {
                channel: 15,
                note: 60,
                velocity: 100
            })
        );
    }
}
