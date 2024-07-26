#[derive(Clone, Copy)]
pub(crate) struct TimeSignature {
    /// Enumerator.
    pub(crate) num_beats: u32,
    /// Denominator.
    pub(crate) note_value: u32,
}

/// Counts offset at a specific measure and subdivison.
pub(crate) struct TimeSignaturesOffsets {
    /// Offsets in milliseconds indexed by measure,
    /// with each entry being (measure start offset, measure duration).
    /// XXX: Also support mid measure tempo changes.
    offsets: Vec<(f32, f32)>,
    // time_signatures: Vec<TimeSignature>,
}

impl TimeSignaturesOffsets {
    pub(crate) fn new(
        time_signatures: &[(usize, TimeSignature)],
        music_offset: f32,
        num_measures: usize,
        bpm: u32,
    ) -> Self {
        // Very unfortunate impure functions.
        let mut current_time_signature = 0;
        let all_measures_time_signatures = (0..num_measures)
            .into_iter()
            .map(|i| {
                let next_index = current_time_signature + 1;
                if next_index < time_signatures.len() && time_signatures[next_index].0 <= i {
                    current_time_signature = next_index;
                }
                time_signatures[current_time_signature].1
            })
            .collect::<Vec<_>>();

        let mut current_measure_offset = 0.0;
        println!("music offset {}", music_offset);

        let offsets = all_measures_time_signatures
            .into_iter()
            .map(|time_signature| {
                let beat_duration = 60.0 / bpm as f32;
                let measure_duration = time_signature.num_beats as f32 * beat_duration;
                let measure_offset = current_measure_offset;

                current_measure_offset += measure_duration;

                (measure_offset, measure_duration)
            })
            .collect::<Vec<_>>();

        Self {
            offsets,
            // time_signatures: all_measures_time_signatures,
        }
    }

    pub(crate) fn offset_at_measure_and_subdivision(
        &self,
        measure: usize,
        subdivision_index: u32,
        subdivision: u32,
    ) -> f32 {
        let (offset, duration) = self.offsets[measure];
        if (measure < 5) {
            println!("offset {} duration {}", offset, duration);
        }

        offset + ((subdivision_index as f32 / subdivision as f32) as f32 * duration)
    }

    pub(crate) fn duration_at_measure(&self, measure: usize) -> f32 {
        self.offsets[measure].1
    }
}
