use std::{collections::HashMap, iter::Enumerate};

use anyhow::Result;

use crate::{
    util::{TimeSignature, TimeSignaturesOffsets},
    Chart, ChartInfo, ChartMode, HitObject, HitObjectParameters, TimingPoint,
    TimingPointBeatLength,
};

use super::read_lines;

///
/// Format specification obtained from
/// https://github.com/kshootmania/ksm-chart-format/blob/master/ksh_format.md.
///

#[derive(Default)]
enum KshNoteType {
    #[default]
    Empty,
    Chip,
    Long,
}

impl KshNoteType {
    fn from_bt_value(value: u8) -> Self {
        match value {
            0 => Self::Empty,
            1 => Self::Chip,
            2 => Self::Long,
            _ => Self::Empty,
        }
    }
    fn from_fx_value(value: u8) -> Self {
        match value {
            0 => Self::Empty,
            1 => Self::Long,
            2 => Self::Chip,
            _ => Self::Long,
        }
    }
}

#[derive(Default)]
enum KshLaserInstance {
    #[default]
    Empty,
    Connection,

    /// A character representing the laser position can be one of the following 51 steps
    /// (from left to right):
    /// Left <- 0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmno -> Right
    Position(char),
}

#[derive(Default)]
enum KshSpinType {
    #[default]
    NormalLeft,
    NormalRight,
    HalfLeft,
    HalfRight,
    SwingLeft,
    SwingRight,
}

#[derive(Default)]
struct KshLaneSpin {
    spin_type: KshSpinType,
    length: u32,
}

const NUM_BT_LANES: usize = 4;
const NUM_FX_LANES: usize = 2;
const NUM_LASER_LANES: usize = 2;

#[derive(Default)]
struct KshNoteLine {
    bt_lanes: [KshNoteType; NUM_BT_LANES],
    fx_lanes: [KshNoteType; NUM_FX_LANES],
    laser_lanes: [KshLaserInstance; NUM_LASER_LANES],
    lane_spin: Option<KshSpinType>,
}

fn bt_notes_to_hit_objects(bt_notes: &[KshNoteType; 4], time: f32) -> Vec<HitObject> {
    let mut hit_objects = Vec::new();
    for i in 0..NUM_BT_LANES {
        match bt_notes[i] {
            // XXX: Only handle chips for now.
            KshNoteType::Chip => {
                hit_objects.push(HitObject {
                    position: (i as _, 0.0),
                    time,
                    object_parameters: HitObjectParameters::Note,
                });
            }
            _ => {}
        }
    }

    hit_objects
}

#[derive(Default)]
enum KshLine {
    #[default]
    Bar,
    Note(KshNoteLine),
    /// Some options are time/position specific.
    Option((String, String)),
}

#[derive(Default, Debug)]
struct KshHeader {
    audio_filenames: Vec<String>,
    /// Offset to start of the audio in milliseconds.
    audio_offset: u32,
    bpm: u32,
    bg_filenames: Vec<String>,
    video_filename: String,
    filter_gain: u32,
    filter_delay: u32,
}

/// Raw chart data, as specified by the format.
#[derive(Default)]
struct KshChartData {
    header: KshHeader,
    body: Vec<KshLine>,
}

const KSH_COMMENT_STR: &str = "//";

fn option_get<'a>(map: &'a HashMap<String, String>, key: &str) -> Result<&'a str> {
    match map.get(key) {
        Some(value) => Ok(value),
        None => Err(anyhow::anyhow!("Key '{}' not found in the map", key)),
    }
}

pub struct KshParser;

impl KshParser {
    pub fn parse_file(file_name: &str) -> Result<Chart> {
        Self::parse_ksh_chart_data(Self::read_to_ksh_data(file_name)?)
    }

    fn parse_ksh_chart_data(ksh_data: KshChartData) -> Result<Chart> {
        let mut current_measure = 0;

        // Separate all bars and time signatures.
        let mut bars = Vec::new();
        let mut time_signatures = Vec::new();
        for chart_line in ksh_data.body {
            match chart_line {
                KshLine::Bar => {
                    current_measure = bars.len();
                    bars.push(Vec::new());
                }
                KshLine::Option((key, value)) => {
                    if key == "beat" {
                        let beat_values = value.split("/").collect::<Vec<_>>();
                        let num_beats = beat_values[0].parse()?;
                        let note_value = beat_values[1].parse()?;

                        time_signatures.push((
                            current_measure,
                            TimeSignature {
                                num_beats,
                                note_value,
                            },
                        ));
                    }
                    // XXX TODO: detect other options, eg tempo changes.
                }
                KshLine::Note(note) => {
                    if let Some(current_bar) = bars.last_mut() {
                        current_bar.push(note);
                    }
                }
            }
        }

        let offset_counter = TimeSignaturesOffsets::new(
            &time_signatures,
            ksh_data.header.audio_offset as f32 / 1000.0,
            bars.len(),
            ksh_data.header.bpm as _,
        );

        let timing_points = time_signatures
            .into_iter()
            .map(|(measure, time_signature)| TimingPoint {
                start_time: (offset_counter.offset_at_measure_and_subdivision(measure, 0, 1)
                    * 1000.0) as u32,
                beat_length: TimingPointBeatLength::Duration(
                    offset_counter.duration_at_measure(measure) * 1000.0 as f32,
                ),
                meter: time_signature.num_beats,
            })
            .collect::<Vec<_>>();

        let hit_objects = bars
            .into_iter()
            .enumerate()
            .map(|(measure, notes)| {
                let subdivision = notes.len();
                notes
                    .into_iter()
                    .enumerate()
                    .map(|(subdivision_index, note)| {
                        let time = offset_counter.offset_at_measure_and_subdivision(
                            measure,
                            subdivision_index as _,
                            subdivision as _,
                        );
                        // if measure < 5 {
                        //     println!(
                        //         "Time at measure {} subdiv {}/{} is {}",
                        //         measure, subdivision_index, subdivision, time
                        //     );
                        // }

                        bt_notes_to_hit_objects(&note.bt_lanes, time * 1000.0)
                    })
                    .flatten()
                    .collect::<Vec<_>>()
            })
            .flatten()
            .collect::<Vec<_>>();

        let info = ChartInfo {
            audio_file_name: ksh_data.header.audio_filenames[0].clone(),
            audio_lead_in: ksh_data.header.audio_offset,
            mode: ChartMode::FixedColumns(4),
        };

        Ok(Chart {
            info,
            timing_points,
            hit_objects,
            playfield: Default::default(),
        })
    }

    fn read_to_ksh_data(file_name: &str) -> Result<KshChartData> {
        let mut body = Vec::new();

        // Contains both global options and positio specific options,
        // but we only care about the global options.
        let mut all_options = HashMap::new();

        let lines = read_lines(file_name)?;
        for line in lines.flatten() {
            let chart_line = Self::parse_line(&line);
            if let Some(chart_line) = chart_line {
                if let KshLine::Option((key, value)) = &chart_line {
                    println!("{}", line);
                    all_options.insert(key.clone(), value.clone());
                }
                body.push(chart_line);
            }
        }

        let header = Self::create_header_from_options(&all_options)?;

        log::info!("Parsed KSH header {:?}", &header);
        log::info!("Parsed KSH body with len {}", body.len());

        Ok(KshChartData { header, body })
    }

    fn create_header_from_options(options: &HashMap<String, String>) -> Result<KshHeader> {
        let audio_filenames = option_get(&options, "m")?
            .split(",")
            .map(|s| s.to_string())
            .collect::<Vec<_>>();
        let audio_offset = option_get(&options, "o")?.parse().map_err(|e| e)?;
        let bpm = option_get(&options, "t")?.parse().map_err(|e| e)?;

        Ok(KshHeader {
            audio_filenames,
            audio_offset,
            bpm,
            ..Default::default()
        })
    }

    fn parse_line(line: &str) -> Option<KshLine> {
        if line.starts_with(KSH_COMMENT_STR) {
            None
        } else if line == "--" {
            Some(KshLine::Bar)
        } else if line.contains("=") {
            Some(KshLine::Option(Self::parse_option(line)))
        } else if line.contains("|") {
            Some(KshLine::Note(Self::parse_note(line)))
        } else {
            None
        }
    }

    fn parse_option(line: &str) -> (String, String) {
        let parts = line.split('=').collect::<Vec<_>>();
        if parts.len() >= 2 {
            (parts[0].to_string(), parts[1].to_string())
        } else {
            (line.to_string(), "".to_string())
        }
    }

    fn parse_note(line: &str) -> KshNoteLine {
        let parts = line.split('|').collect::<Vec<_>>();

        // XXX: Properly verify line format?

        let bt_notes = [
            parts[0][0..1].parse().unwrap_or(0),
            parts[0][1..2].parse().unwrap_or(0),
            parts[0][2..3].parse().unwrap_or(0),
            parts[0][3..4].parse().unwrap_or(0),
        ];
        let fx_notes = [
            parts[1][0..1].parse().unwrap_or(0),
            parts[1][1..2].parse().unwrap_or(0),
        ];

        let bt_lanes = [
            KshNoteType::from_bt_value(bt_notes[0]),
            KshNoteType::from_bt_value(bt_notes[1]),
            KshNoteType::from_bt_value(bt_notes[2]),
            KshNoteType::from_bt_value(bt_notes[3]),
        ];
        let fx_lanes = [
            KshNoteType::from_fx_value(fx_notes[0]),
            KshNoteType::from_fx_value(fx_notes[1]),
        ];

        KshNoteLine {
            bt_lanes,
            fx_lanes,
            laser_lanes: Default::default(),
            lane_spin: None,
        }
    }
}
