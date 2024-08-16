use std::collections::HashMap;

use anyhow::{anyhow, Result};
use regex::Regex;

use crate::{parse::read_lines, util::MeasureData, Chart, Header, TimeSignature};

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

struct BodyLine {
    body_type: String,
    beat: Vec<String>,
    position: Vec<String>,
    additional_options: Option<String>,
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
                            match body_line.body_type {
                                _ => log::warn!(
                                    "Unregocnized chart body type {}",
                                    body_line.body_type
                                ),
                            }
                        } else {
                            log::warn!("Unrecognized chart body line {}", line);
                        }
                    }
                }
            }
        }

        Ok(Chart {
            header,
            platforms,
            notes,
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
            let additional_options = caps
                .name("additional_options")
                .map(|m| m.as_str().trim().to_string());

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
}
