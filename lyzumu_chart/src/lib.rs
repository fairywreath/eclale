pub mod parse;
mod util;

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct TimeSignature {
    /// Enumerator.
    pub(crate) num_beats: u32,
    /// Denominator.
    pub(crate) note_value: u32,
}

pub struct TimingPoint {
    measure: (u32, f32),
    pub seconds: Option<f32>,
    pub z_position: Option<f32>,
}

#[derive(Default)]
pub struct Header {
    pub audio_filename: String,
    /// Offset to start of audio in seconds.
    pub audio_offset: f32,

    pub(crate) default_tempo: u32,
    pub(crate) default_time_signature: TimeSignature,
}

pub struct BezierControlPoint {
    pub x_position: f32,
    pub time: TimingPoint,
}

pub struct Platform {
    pub start_time: TimingPoint,
    pub end_time: TimingPoint,

    /// Vertices for the "quad" platform in order of
    /// bottom_left, bottom_right, top_left, top_right.
    pub vertices_x_positions: (f32, f32, f32, f32),

    // Left and right bezier control points.
    pub control_points: (Option<BezierControlPoint>, Option<BezierControlPoint>),
}

impl Platform {
    pub fn is_quad(&self) -> bool {
        self.control_points.0.is_none() && self.control_points.1.is_none()
    }
}

pub enum BasicNoteType {
    Basic1,
    Basic2,
    Basic3,
    Basic4,
}

pub enum EvadeNoteType {
    Evade1,
    Evade2,
    Evade3,
    Evade4,
}

pub enum ContactNoteType {
    Contact1,
    Contact2,
}

pub struct HoldNote {
    pub end_time: TimingPoint,
    pub control_points: (Option<BezierControlPoint>, Option<BezierControlPoint>),
}

pub enum NoteData {
    Basic(BasicNoteType),
    BasicHold((BasicNoteType, HoldNote)),
    Target,
    TargetHold(HoldNote),
    Evade(EvadeNoteType),
    Contact(ContactNoteType),
    Floor,
}

pub struct Note {
    pub data: NoteData,
    pub time: TimingPoint,
}

pub struct Chart {
    pub header: Header,
    pub platforms: Vec<Platform>,
    pub notes: Vec<Note>,
}
