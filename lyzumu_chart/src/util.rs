use crate::TimeSignature;

/// Contains all required musical and rhythm data for a single measure/bar to calculate raw time offsets.
#[derive(Default, Clone, Debug)]
pub(crate) struct MeasureData {
    pub(crate) time_signature: TimeSignature,
    pub(crate) tempo: u32,

    /// Subdivision the notes in the measure are written against.
    pub(crate) subdivision: u32,
}

/// Counts offset at a specific measure and subdivison.
pub(crate) struct TimeSignaturesOffsets {
    measures: Vec<MeasureData>,

    /// Offsets in seconds indexed by measure,
    /// with each entry being (measure start offset, measure duration).
    measure_offsets: Vec<(f32, f32)>,
}

impl TimeSignaturesOffsets {
    pub(crate) fn new(measures: Vec<MeasureData>, music_offset: f32) -> Self {
        println!("Measure data: {:#?}", &measures);

        let mut current_measure_offset = music_offset;
        let measure_offsets = measures
            .iter()
            .map(|measure| {
                let time_signature = measure.time_signature;
                let bpm = measure.tempo;

                let beat_duration = 60.0 / bpm as f32;

                let measure_duration = time_signature.num_beats as f32
                    * beat_duration
                    // BPM is measured in quarter notes, i.e. `4.0`.
                    * (4.0 / time_signature.note_value as f32);

                let measure_offset = current_measure_offset;

                current_measure_offset += measure_duration;

                (measure_offset, measure_duration)
            })
            .collect::<Vec<_>>();
        Self {
            measures,
            measure_offsets,
        }
    }

    /// Returns offset in seconds.
    pub(crate) fn offset_at_measure(&self, measure: usize, subdivision_index: f32) -> f32 {
        let (offset, duration) = self.measure_offsets[measure];

        // Index starts at 1.
        let index = subdivision_index as f32 - 1.0;
        let subdivision = self.measures[measure].subdivision;
        offset + (index / subdivision as f32) as f32 * duration
    }

    /// Returns duration in seconds.
    pub(crate) fn duration_at_measure(&self, measure: usize) -> f32 {
        self.measure_offsets[measure].1
    }
}
