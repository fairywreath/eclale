use anyhow::{anyhow, Result};
use regex::Regex;

use crate::{
    parse::read_lines,
    util::{MeasureData, TimeSignaturesOffsets},
    BasicNoteType, BezierControlPoint, Chart, ContactNoteType, EvadeNoteType, FlickDirection,
    FlickNote, Header, HoldNote, Note, NoteData, Platform, TimeSignature, TimingPoint,
};

enum Section {
    Header,
    ChartBody,
}

impl TryFrom<&str> for Section {
    type Error = anyhow::Error;

    fn try_from(s: &str) -> Result<Self> {
        match s {
            "header" => Ok(Self::Header),
            "chart_body" => Ok(Self::ChartBody),
            _ => Err(anyhow!("Invalid string for section tag conversion: {}", s)),
        }
    }
}

impl TryFrom<&str> for BasicNoteType {
    type Error = anyhow::Error;

    fn try_from(s: &str) -> Result<Self> {
        match s {
            "B1" | "HB1" => Ok(Self::Basic1),
            "B2" | "HB2" => Ok(Self::Basic2),
            "B3" | "HB3" => Ok(Self::Basic3),
            "B4" | "HB4" => Ok(Self::Basic4),
            _ => Err(anyhow!("Invalid string for section tag conversion: {}", s)),
        }
    }
}

impl TryFrom<&str> for EvadeNoteType {
    type Error = anyhow::Error;

    fn try_from(s: &str) -> Result<Self> {
        match s {
            "E1" => Ok(Self::Evade1),
            "E2" => Ok(Self::Evade2),
            "E3" => Ok(Self::Evade3),
            "E4" => Ok(Self::Evade4),
            _ => Err(anyhow!("Invalid string for section tag conversion: {}", s)),
        }
    }
}

impl TryFrom<&str> for ContactNoteType {
    type Error = anyhow::Error;

    fn try_from(s: &str) -> Result<Self> {
        match s {
            "C1" => Ok(Self::Contact1),
            "C2" => Ok(Self::Contact2),
            _ => Err(anyhow!("Invalid string for section tag conversion: {}", s)),
        }
    }
}

struct BodyLine {
    body_type: String,
    beat: Vec<String>,
    position: Vec<String>,
    additional_options: String,
}

pub struct LzmParser {
    regex_section_tag: Regex,
    regex_option: Regex,
    regex_body_line: Regex,
}

impl LzmParser {
    pub fn new() -> Result<Self> {
        Ok(Self {
            regex_section_tag: Regex::new(r"<(.*?)>")?,
            regex_option: Regex::new(r"(\w+)=(\w+)")?,
            regex_body_line: Regex::new(
                r"\[(?P<body_type>[^\]]+)\] \((?P<beat>[^\)]+)\) \|(?P<position>[^\|]+)\|(?: \{(?P<additional_options>[^\}]*)\})?",
            )?,
        })
    }

    pub fn parse_file(&self, filename: &str) -> Result<Chart> {
        let mut header = Header::default();
        let mut notes = Vec::new();
        let mut platforms = Vec::new();

        let mut current_section = Section::Header;
        let mut current_measure_data = MeasureData::default();
        let mut measures = Vec::new();

        for line in read_lines(filename)?.flatten() {
            let line = line.trim();

            if let Some(new_section) = self.parse_section_tag(line) {
                current_section = new_section
            } else {
                match current_section {
                    Section::Header => {
                        if let Some((key, value)) = self.parse_option(line).into_iter().next() {
                            match key.as_str() {
                                "audio_filename" => header.audio_filename = value,
                                "audio_offset" => header.audio_offset = value.parse()?,
                                "default_tempo" => {
                                    header.default_tempo = value.parse()?;
                                    current_measure_data.tempo = header.default_tempo;
                                }
                                "default_time_signature" => {
                                    header.default_time_signature =
                                        Self::parse_time_signature(&value)?;
                                    current_measure_data.time_signature =
                                        header.default_time_signature;
                                }
                                _ => {
                                    log::warn!("Unrecognized header option key {}", key);
                                }
                            }
                        } else {
                            log::warn!("Unrecognized header line {}", line);
                        }
                    }
                    Section::ChartBody => {
                        if line == "--" {
                            measures.push(current_measure_data.clone());
                        } else if let Some((key, value)) =
                            self.parse_option(line).into_iter().next()
                        {
                            match key.as_str() {
                                "time_signature" => {
                                    current_measure_data.time_signature =
                                        Self::parse_time_signature(&value)?
                                }
                                "tempo" => current_measure_data.tempo = value.parse()?,
                                "subdivision" => {
                                    current_measure_data.subdivision = value.parse()?
                                }
                                _ => log::warn!("Unrecognized chart body option key {}", key),
                            }
                        } else if let Some(body_line) = self.parse_body_line(line) {
                            match body_line.body_type.as_str() {
                                "PQ" | "PC" => {
                                    platforms.push(Self::parse_platform(
                                        &body_line,
                                        measures.len() as _,
                                    )?);
                                }
                                _ => {
                                    notes.push(Self::parse_note(&body_line, measures.len() as _)?);
                                }
                            }
                        } else {
                            log::warn!("Unrecognized chart body line {}", line);
                        }
                    }
                }
            }
        }

        let offset_counter = TimeSignaturesOffsets::new(measures, header.audio_offset);
        let notes = Self::count_note_offsets(&offset_counter, notes);
        let platforms = Self::count_platform_offsets(&offset_counter, platforms);

        Ok(Chart {
            header,
            platforms,
            notes,
        })
    }

    fn count_note_offsets(offset_counter: &TimeSignaturesOffsets, notes: Vec<Note>) -> Vec<Note> {
        notes
            .into_iter()
            .map(|n| {
                let time = Self::count_offset_seconds(offset_counter, &n.time);
                let data = match n.data {
                    NoteData::BasicHold((note_type, hold_note)) => NoteData::BasicHold((
                        note_type,
                        Self::count_hold_note_offset(offset_counter, hold_note),
                    )),
                    NoteData::TargetHold(hold_note) => NoteData::TargetHold(
                        Self::count_hold_note_offset(offset_counter, hold_note),
                    ),
                    NoteData::FloorHold(hold_note) => {
                        NoteData::FloorHold(Self::count_hold_note_offset(offset_counter, hold_note))
                    }
                    _ => n.data,
                };
                Note {
                    data,
                    time,
                    x_position: n.x_position,
                }
            })
            .collect()
    }

    fn count_hold_note_offset(
        offset_counter: &TimeSignaturesOffsets,
        mut hold_note: HoldNote,
    ) -> HoldNote {
        let end_time = Self::count_offset_seconds(offset_counter, &hold_note.end_time);
        let control_points = hold_note
            .control_points
            .into_iter()
            .map(|c| {
                if let Some(control_point) = c {
                    Some(Self::count_bezier_control_point_offset(
                        offset_counter,
                        control_point,
                    ))
                } else {
                    c
                }
            })
            .collect();

        hold_note.end_time = end_time;
        hold_note.control_points = control_points;
        hold_note
    }

    fn count_bezier_control_point_offset(
        offset_counter: &TimeSignaturesOffsets,
        mut control_point: BezierControlPoint,
    ) -> BezierControlPoint {
        let time = Self::count_offset_seconds(offset_counter, &control_point.time);
        control_point.time = time;
        control_point
    }

    fn count_platform_offsets(
        offset_counter: &TimeSignaturesOffsets,
        platforms: Vec<Platform>,
    ) -> Vec<Platform> {
        platforms
            .into_iter()
            .map(|p| {
                let start_time = Self::count_offset_seconds(offset_counter, &p.start_time);
                let end_time = Self::count_offset_seconds(offset_counter, &p.end_time);
                p.timing_points(start_time, end_time)
            })
            .collect()
    }

    /// Creates new `TimingPoint` with the `seconds` entry populated.
    fn count_offset_seconds(
        offset_counter: &TimeSignaturesOffsets,
        time: &TimingPoint,
    ) -> TimingPoint {
        let seconds = offset_counter.offset_at_measure(time.measure.0 as _, time.measure.1);
        time.clone().seconds(seconds)
    }

    fn parse_note(body_line: &BodyLine, current_measure: u32) -> Result<Note> {
        let time = Self::parse_time(&body_line.beat[0], current_measure)?;
        match body_line.body_type.as_str() {
            "B1" | "B2" | "B3" | "B4" => Ok(Note {
                data: NoteData::Basic(body_line.body_type.as_str().try_into()?),
                time,
                x_position: body_line.position[0].parse()?,
            }),
            "T" => Ok(Note {
                data: NoteData::Target,
                time,
                x_position: body_line.position[0].parse()?,
            }),
            "E1" | "E2" | "E3" | "E4" => Ok(Note {
                data: NoteData::Evade(body_line.body_type.as_str().try_into()?),
                time,
                x_position: body_line.position[0].parse()?,
            }),
            "C1" | "C2" => Ok(Note {
                data: NoteData::Contact(body_line.body_type.as_str().try_into()?),
                time,
                x_position: body_line.position[0].parse()?,
            }),
            "FO" => Ok(Note {
                data: NoteData::Floor,
                time,
                x_position: 0.0,
            }),
            "FL" | "FR" => {
                let direction = if body_line.body_type == "FL" {
                    FlickDirection::Left
                } else {
                    FlickDirection::Right
                };
                Ok(Note {
                    data: NoteData::Flick(FlickNote {
                        direction,
                        end_x_position: body_line.additional_options.parse()?,
                    }),
                    time,
                    x_position: 0.0,
                })
            }
            "HB1" | "HB2" | "HB3" | "HB4" => Ok(Note {
                data: NoteData::BasicHold((
                    body_line.body_type.as_str().try_into()?,
                    Self::parse_hold_note(body_line, current_measure)?,
                )),
                time,
                x_position: 0.0,
            }),
            "HT" => Ok(Note {
                data: NoteData::TargetHold(Self::parse_hold_note(body_line, current_measure)?),
                time,
                x_position: 0.0,
            }),
            "HF" => Ok(Note {
                data: NoteData::FloorHold(Self::parse_hold_note(body_line, current_measure)?),
                time,
                x_position: 0.0,
            }),
            _ => Err(anyhow!(
                "Unrecognizable chart body type {}",
                body_line.body_type
            )),
        }
    }

    fn parse_hold_note(body_line: &BodyLine, current_measure: u32) -> Result<HoldNote> {
        Ok(HoldNote {
            end_time: Self::parse_time(&body_line.beat[1], current_measure)?,
            control_points: Self::parse_2_control_points(
                &body_line.additional_options,
                current_measure,
            )?,
        })
    }

    fn parse_section_tag(&self, input: &str) -> Option<Section> {
        let matches = self
            .regex_section_tag
            .captures_iter(input)
            .filter_map(|caps| caps.get(1))
            .map(|m| m.as_str().to_string())
            .collect::<Vec<_>>();

        if !matches.is_empty() {
            matches[0].as_str().try_into().ok()
        } else {
            None
        }
    }

    fn parse_option(&self, input: &str) -> Vec<(String, String)> {
        let mut pairs = Vec::new();

        for caps in self.regex_option.captures_iter(input) {
            if let (Some(key), Some(value)) = (caps.get(1), caps.get(2)) {
                pairs.push((key.as_str().to_string(), value.as_str().to_string()));
            }
        }

        pairs
    }

    fn parse_time_signature(input: &str) -> Result<TimeSignature> {
        // Split the input string by '/'
        let parts: Vec<&str> = input.split('/').collect();

        // Ensure that we have exactly two parts
        if parts.len() != 2 {
            return Err(anyhow::anyhow!(
                "Invalid time signature format: expected 'numerator/denominator'"
            ));
        }

        let time_signature = TimeSignature {
            num_beats: parts[0].parse()?,
            note_value: parts[1].parse()?,
        };

        Ok(time_signature)
    }

    fn parse_body_line(&self, input: &str) -> Option<BodyLine> {
        if let Some(caps) = self.regex_body_line.captures(input) {
            let body_type = caps.name("body_type")?.as_str().trim().to_string();
            let beat = caps
                .name("beat")?
                .as_str()
                .split(';')
                .map(|s| s.trim().to_string())
                .collect();
            let position = caps
                .name("position")?
                .as_str()
                .split(';')
                .map(|s| s.trim().to_string())
                .collect();
            let additional_options = caps.name("additional_options")?.as_str().to_string();

            Some(BodyLine {
                body_type,
                beat,
                position,
                additional_options,
            })
        } else {
            None
        }
    }

    fn parse_time(input: &str, current_measure: u32) -> Result<TimingPoint> {
        let inputs = input.split(",").collect::<Vec<_>>();
        if inputs.len() > 1 {
            let beat = inputs[0].parse()?;
            let delta_measure = inputs[1].parse::<u32>()?;

            Ok(TimingPoint::new_measure(
                current_measure + delta_measure,
                beat,
            ))
        } else {
            let beat = inputs[0].parse()?;
            Ok(TimingPoint::new_measure(current_measure, beat))
        }
    }

    fn parse_platform(body_line: &BodyLine, current_measure: u32) -> Result<Platform> {
        let platform = Platform {
            start_time: Self::parse_time(&body_line.beat[0], current_measure)?,
            end_time: Self::parse_time(&body_line.beat[1], current_measure)?,
            vertices_x_positions: (
                body_line.position[0].parse()?,
                body_line.position[1].parse()?,
                body_line.position[2].parse()?,
                body_line.position[3].parse()?,
            ),
            control_points: if !body_line.additional_options.is_empty() {
                Self::parse_control_points(&body_line.additional_options, current_measure)?
            } else {
                Vec::new()
            },
        };

        Ok(platform)
    }

    fn parse_control_points(
        input: &str,
        current_measure: u32,
    ) -> Result<Vec<Option<BezierControlPoint>>> {
        if input.is_empty() {
            return Ok(Vec::new());
        }

        let inputs = input.split(";").collect::<Vec<_>>();

        let mut points = Vec::new();
        points.extend(Self::parse_2_control_points(&inputs[0], current_measure)?);
        points.extend(Self::parse_2_control_points(&inputs[1], current_measure)?);

        Ok(points)
    }

    fn parse_2_control_points(
        input: &str,
        current_measure: u32,
    ) -> Result<Vec<Option<BezierControlPoint>>> {
        let mut points = Vec::new();

        let inputs = input.split(":").collect::<Vec<_>>();
        if !inputs.is_empty() {
            let point_1_inputs = inputs[0].split_once(",");
            let point_2_inputs = inputs[1].split_once(",");
            points.push(Self::parse_control_point(point_1_inputs, current_measure)?);
            points.push(Self::parse_control_point(point_2_inputs, current_measure)?);
        } else {
            points.push(None);
            points.push(None);
        }

        Ok(points)
    }

    fn parse_control_point(
        inputs: Option<(&str, &str)>,
        current_measure: u32,
    ) -> Result<Option<BezierControlPoint>> {
        if let Some((x_position_str, time_str)) = &inputs {
            Ok(Some(BezierControlPoint {
                x_position: x_position_str.parse()?,
                time: Self::parse_time(time_str, current_measure)?,
            }))
        } else {
            Ok(None)
        }
    }
}
