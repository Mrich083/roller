use num_derive::FromPrimitive;
use num_traits::FromPrimitive;

/// Borrowed from https://github.com/RustAudio/rimd/blob/54fd9bd2bd3caaa6fe1c31fbf71c0f3c6597fd1a/src/midi.rs#L51-L77
/// The status field of a midi message indicates what midi command it
/// represents and what channel it is on
#[derive(Debug, PartialEq, Clone, Copy, FromPrimitive)]
pub enum Status {
    // voice
    NoteOff = 0x80,
    NoteOn = 0x90,
    PolyphonicAftertouch = 0xA0,
    ControlChange = 0xB0,
    ProgramChange = 0xC0,
    ChannelAftertouch = 0xD0,
    PitchBend = 0xE0,

    // sysex
    SysExStart = 0xF0,
    MIDITimeCodeQtrFrame = 0xF1,
    SongPositionPointer = 0xF2,
    SongSelect = 0xF3,
    TuneRequest = 0xF6, // F4 anf 5 are reserved and unused
    SysExEnd = 0xF7,
    TimingClock = 0xF8,
    Start = 0xFA,
    Continue = 0xFB,
    Stop = 0xFC,
    ActiveSensing = 0xFE, // FD also res/unused
    SystemReset = 0xFF,
}

pub const STATUS_MASK: u8 = 0xF0;

#[derive(Debug, Clone, PartialEq)]
pub enum MidiEvent {
    NoteOn { note: u8, velocity: u8 },
    NoteOff { note: u8, velocity: u8 },
    ControlChange { control: u8, value: u8 },
    Other(Status),
}
impl MidiEvent {
    pub fn from_bytes(bytes: &[u8]) -> MidiEvent {
        let status = Status::from_u8(bytes[0] & STATUS_MASK).unwrap();

        match status {
            Status::NoteOn => MidiEvent::NoteOn {
                note: bytes[1],
                velocity: bytes[2],
            },
            Status::NoteOff => MidiEvent::NoteOff {
                note: bytes[1],
                velocity: bytes[2],
            },
            Status::ControlChange => MidiEvent::ControlChange {
                control: bytes[1],
                value: bytes[2],
            },
            _ => MidiEvent::Other(status),
        }
    }
}