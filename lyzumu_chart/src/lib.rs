pub mod parse;
mod util;

///
/// Chart format is based on osu!mania's beatmap file format.
/// This is a "low-level"/"raw" chart format, in a sense that every timing/beat is specified by
/// a raw timed offset(eg. milliseconds) from the beginning of the song instead of higher-level
/// constructs such as beats and measures.
///

/// Represents time signatures.
pub struct TimingPoint {
    /// Start time in ms from the beginning of the audio.
    /// XXX: change this to floating point.
    pub start_time: u32,

    /// Either beat duration in ms, or multiplier if inherited.
    pub beat_length: TimingPointBeatLength,

    /// Number of beats in a measure.
    pub meter: u32,
}

pub enum TimingPointBeatLength {
    Duration(f32),
}

pub struct HitObject {
    pub position: (f32, f32),

    /// Time in milliseconds from the start of the audio.
    pub time: f32,

    /// Object type additional parameters, may contain additional parameters
    pub object_parameters: HitObjectParameters,
}

pub enum HitObjectParameters {
    Note,

    /// Contains end time in ms from the start of the audio.
    HoldNote(f32),
}

pub struct ChartInfo {
    /// File path to raw audio file.
    pub audio_file_name: String,

    /// Offset in ms before the audio starts playing.
    pub audio_lead_in: i32,

    /// Contains game mode specific parameters.
    pub mode: ChartMode,
}

pub enum ChartMode {
    /// Classic fixed number of columns, eg. 4K, 7K, etc.
    /// Contains number of columns.
    FixedColumns(u32),
}

#[derive(Default)]
pub struct Playfield {
    /// Default hit object speed.
    pub default_speed: f32,
}

pub struct Chart {
    pub info: ChartInfo,
    pub timing_points: Vec<TimingPoint>,
    pub hit_objects: Vec<HitObject>,
    pub playfield: Playfield,
}
