use std::collections::HashMap;

use anyhow::Result;

use crate::{types::TimeSignature, Chart};

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

#[derive(Default)]
struct KshNoteLine {
    bt_lanes: [KshNoteType; 4],
    fx_lanes: [KshNoteType; 2],
    laser_lanes: [KshLaserInstance; 2],
    lane_spin: Option<KshSpinType>,
}

#[derive(Default)]
enum KshChartLine {
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

    bg_filenames: Vec<String>,
    video_filename: String,
    filter_gain: u32,
    filter_delay: u32,
}

/// Raw chart data, as specified by the format.
#[derive(Default)]
struct KshChartData {
    header: KshHeader,
    body: Vec<KshChartLine>,
}

/// Pre-processed chart data into more understandable format.
struct KshProcessedChartData {
    /// Must be ordered by measure count.
    time_signatures: Vec<(usize, TimeSignature)>,
    /// Vector size of each bar determines subdivision of the individual notes.
    bars: Vec<Vec<KshNoteLine>>,
    // XXX: State on each bar, etc.
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
        let raw_chart_data = Self::read_to_ksh_data(file_name)?;

        todo!()
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
                if let KshChartLine::Option((key, value)) = &chart_line {
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

        Ok(KshHeader {
            audio_filenames,
            audio_offset,
            ..Default::default()
        })
    }

    fn parse_line(line: &str) -> Option<KshChartLine> {
        if line.starts_with(KSH_COMMENT_STR) {
            None
        } else if line == "--" {
            Some(KshChartLine::Bar)
        } else if line.contains("=") {
            Some(KshChartLine::Option(Self::parse_option(line)))
        } else if line.contains("|") {
            Some(KshChartLine::Note(Self::parse_note(line)))
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
