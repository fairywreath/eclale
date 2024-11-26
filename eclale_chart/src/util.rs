use std::ops::Add;

use crate::Time;

use super::TimeSignature;

/// Contains all required musical and rhythm data for a single measure/bar to calculate raw time offsets.
#[derive(Default, Clone, Debug)]
pub(crate) struct MeasureCompositionData {
    pub(crate) time_signature: TimeSignature,
    pub(crate) bpm: u32,
    pub(crate) speed_multiplier: f32,
    /// Subdivision the notes in the measure are written against.
    pub(crate) subdivision: u32,
}

#[derive(Default, Clone, Copy, Debug)]
pub(crate) struct ZPosition {
    pub(crate) time: Time,
    pub(crate) z: f32,
}

impl ZPosition {
    pub(crate) fn new(time: Time, z: f32) -> Self {
        Self { time, z }
    }
}

/// Contains positional/offset data for a measure.
#[derive(Default, Clone, Copy, Debug)]
struct MeasurePositionData {
    offset: ZPosition,
    duration: ZPosition,
}

#[derive(Default, Clone, Debug)]
struct MeasureData {
    composition: MeasureCompositionData,
    position: MeasurePositionData,
}

/// Counts offset at a specific measure and subdivison.
pub(crate) struct ZPositionCalculator {
    /// Sorted by positiona; offset.
    measures: Vec<MeasureData>,
}

impl ZPositionCalculator {
    pub(crate) fn new(
        measure_compositions: Vec<MeasureCompositionData>,
        music_offset: Time,
        z_base_speed: f32,
    ) -> Self {
        let measures = measure_compositions
            .into_iter()
            .scan(
                ZPosition::new(music_offset, music_offset.0 * z_base_speed),
                |current_position, measure| {
                    let time_signature = measure.time_signature;
                    let bpm = measure.bpm;

                    let beat_duration = 60.0 / bpm as f32;
                    let z_duration_time = time_signature.num_beats as f32
                            * beat_duration
                            // BPM is measured in quarter notes, i.e. `4.0`.
                            * (4.0 / time_signature.note_value as f32);

                    // XXX TODO: Handle soflans properly. Soflans can happen multiple times within
                    // a measure as well.
                    let z_duration_length = time_signature.num_beats as f32
                            * beat_duration
                            // BPM is measured in quarter notes, i.e. `4.0`.
                            * (4.0 / time_signature.note_value as f32)
                            * z_base_speed;
                    // * measure.speed_multiplier;

                    let offset = current_position.clone();
                    current_position.time.0 += z_duration_time;
                    current_position.z += z_duration_length;

                    Some(MeasureData {
                        composition: measure,
                        position: MeasurePositionData {
                            offset,
                            duration: ZPosition::new(Time(z_duration_time), z_duration_length),
                        },
                    })
                },
            )
            .collect::<Vec<_>>();

        // println!("{:#?}", &measures);

        Self { measures }
    }

    pub(crate) fn z_position_at(&self, measure: usize, subdivision_index: f32) -> ZPosition {
        let subdivision = self.measures[measure].composition.subdivision;
        let position_data = self.measures[measure].position;

        // Index starts at 1.
        let index = subdivision_index as f32 - 1.0;
        let ratio = (index / subdivision as f32) as f32;

        let time = Time(position_data.offset.time.0 + ratio * position_data.duration.time.0);
        let z = position_data.offset.z + ratio * position_data.duration.z;

        ZPosition::new(time, z)
    }
}

pub(crate) struct XPositionCalculator {
    resolution: f32,
}

impl XPositionCalculator {
    pub(crate) fn new(resolution: f32) -> Self {
        Self { resolution }
    }

    pub(crate) fn x_position_at(&self, abs_position: f32, offset_position: f32) -> f32 {
        // abs_position + offset_position * self.resolution
        // abs_position * self.resolution

        // (abs_position + offset_position * self.resolution) / 16.0

        // XXX  FIXME: Properly adjust this based on chart settings.
        // XXX TODO:
        abs_position / 25.0 + (offset_position / self.resolution / 25.0)
        // abs_position / 25.0
    }
}
