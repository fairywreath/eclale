use std::collections::HashMap;

use util::ZPositionCalculator;

pub mod parse;
mod util;

/// Time in seconds.
#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd)]
pub struct Time(pub f32);

#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd)]
pub struct TrackPosition {
    pub time: Time,
    pub z: f32,
    pub x: f32,
}

/// Contains points(positions) sorted by time.
#[derive(Clone, Debug)]
pub struct Platform {
    pub points_left: Vec<TrackPosition>,
    pub points_right: Vec<TrackPosition>,
}

/// Generic unsigned integer that can be identified differently based on chart type.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct LaneType(pub u16);

#[derive(Clone, Debug)]
pub struct Lane {
    pub points: Vec<TrackPosition>,
}

#[derive(Clone, Debug, Default)]
pub struct Track {
    pub platforms: Vec<Platform>,
    // XXX TODO: Have dedicated type Lanes and Lane.
    pub lanes: HashMap<LaneType, Vec<Lane>>,
}

/// Generic unsigned integer that can be identified differently based on chart type.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HitNoteType(pub u16);

#[derive(Clone, Debug)]
pub struct HitNote {
    pub ty: HitNoteType,
    pub position: TrackPosition,
}

/// Generic unsigned integer that can be identified differently based on chart type.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ContactNoteType(pub u16);

#[derive(Clone, Debug)]
pub struct ContactNote {
    pub ty: ContactNoteType,
    pub position: TrackPosition,
}

/// Generic unsigned integer that can be identified differently based on chart type.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EvadeNoteType(pub u16);

/// Linear translation.
#[derive(Clone, Debug)]
pub struct NoteMovement {
    /// Initial position of the object.
    pub start: TrackPosition,

    /// Final position of the object.
    /// XXX TODO: Support current player position as end position.
    pub end: TrackPosition,

    /// Time when movement begins.
    pub trigger_time: Time,

    /// Duration in seconds.
    pub duration: f32,
}

impl NoteMovement {
    pub fn is_static(&self) -> bool {
        self.start == self.end
    }
}

#[derive(Clone, Debug)]
pub struct EvadeNote {
    pub ty: EvadeNoteType,
    pub movement: NoteMovement,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FlickDirection {
    Left,
    Right,
}

#[derive(Clone, Debug)]
pub struct FlickNote {
    pub direction: FlickDirection,
    pub position: TrackPosition,
}

#[derive(Clone, Debug)]
pub struct HoldNote {
    pub ty: HitNoteType,
    pub points: Vec<TrackPosition>,
}

#[derive(Clone, Debug, Default)]
pub struct Notes {
    pub hits: Vec<HitNote>,
    pub holds: Vec<HoldNote>,
    pub contacts: Vec<ContactNote>,
    pub evades: Vec<EvadeNote>,
    pub flicks: Vec<FlickNote>,
}

#[derive(Clone, Debug)]
pub struct BpmChange {
    pub time: Time,
    pub bpm: u32,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct TimeSignature {
    /// Enumerator.
    pub num_beats: u32,
    /// Denominator.
    pub note_value: u32,
}

#[derive(Clone, Debug)]
pub struct TimeSignatureChange {
    pub time: Time,
    pub time_signature: TimeSignature,
}

#[derive(Clone, Debug)]
pub struct Soflan {
    pub time: Time,
    pub duration: f32,
    pub speed_multiplier: f32,
}

/// Vec members are sorted by time.
#[derive(Clone, Debug, Default)]
pub struct Composition {
    pub bpm_changes: Vec<BpmChange>,
    pub time_signature_changes: Vec<TimeSignatureChange>,
    pub soflans: Vec<Soflan>,
}

#[derive(Clone, Debug, Default)]
pub struct ChartData {
    pub track: Track,
    pub notes: Notes,
    pub composition: Composition,
}

#[derive(Clone, Debug, Default)]
pub struct Header {
    /// Path of audio file.
    pub audio_filename: String,

    /// Offset to start of audio in seconds.
    pub audio_offset: f32,
}

/// Internal chart metadata, not set by chart file.
#[derive(Clone, Debug, Default)]
pub struct Metadata {
    /// Base speed multiplier used to calculate "time".
    pub base_speed: f32,
}

#[derive(Clone, Debug)]
pub struct ChartUtils {
    pub z_position_calculator: ZPositionCalculator,
}

#[derive(Clone, Debug)]
pub struct Chart {
    pub header: Header,
    pub metadata: Metadata,
    pub data: ChartData,
    pub utils: ChartUtils,
}
