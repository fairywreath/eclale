pub mod parse;
mod util;

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct TimeSignature {
    /// Enumerator.
    pub(crate) num_beats: u32,
    /// Denominator.
    pub(crate) note_value: u32,
}

#[derive(Clone)]
pub struct TimingPoint {
    pub(crate) measure: (u32, f32),
    pub seconds: Option<f32>,
    pub z_position: Option<f32>,
}

impl TimingPoint {
    pub(crate) fn seconds(mut self, seconds: f32) -> Self {
        self.seconds = Some(seconds);
        self
    }
}

impl TimingPoint {
    pub(crate) fn new_measure(measure: u32, beat: f32) -> Self {
        Self {
            measure: (measure, beat),
            seconds: None,
            z_position: None,
        }
    }
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
    pub control_points: Vec<Option<BezierControlPoint>>,
}

impl Platform {
    pub fn is_quad(&self) -> bool {
        for point in &self.control_points {
            if point.is_some() {
                return false;
            }
        }
        true
    }

    pub(crate) fn timing_points(mut self, start_time: TimingPoint, end_time: TimingPoint) -> Self {
        self.start_time = start_time;
        self.end_time = end_time;
        self
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
    pub control_points: Vec<Option<BezierControlPoint>>,
}

pub enum FlickDirection {
    Left,
    Right,
}

pub struct FlickNote {
    pub direction: FlickDirection,
    pub end_x_position: f32,
}

pub enum NoteData {
    Basic(BasicNoteType),
    BasicHold((BasicNoteType, HoldNote)),
    Target,
    TargetHold(HoldNote),
    Evade(EvadeNoteType),
    Contact(ContactNoteType),
    Floor,
    FloorHold(HoldNote),
    Flick(FlickNote),
}

pub struct Note {
    pub data: NoteData,
    pub time: TimingPoint,
    pub x_position: f32,
}

pub struct Chart {
    pub header: Header,
    pub platforms: Vec<Platform>,
    pub notes: Vec<Note>,
}
