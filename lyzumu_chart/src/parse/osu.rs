use std::collections::HashMap;

use anyhow::{anyhow, Result};

use crate::{
    parse::read_lines, Chart, ChartInfo, ChartMode, HitObject, HitObjectParameters, Playfield,
    TimingPoint, TimingPointBeatLength,
};

pub struct OsuManiaParser;

enum Tag {
    General,
    Difficulty,
    TimingPoints,
    HitObjects,
    /// Skip/not interested in.
    Other,
}

impl TryFrom<&str> for Tag {
    type Error = anyhow::Error;

    fn try_from(s: &str) -> Result<Self> {
        match s {
            "[General]" => Ok(Tag::General),
            "[Difficulty]" => Ok(Tag::Difficulty),
            "[TimingPoints]" => Ok(Tag::TimingPoints),
            "[HitObjects]" => Ok(Tag::HitObjects),
            _ => Ok(Tag::Other),
        }
    }
}

impl OsuManiaParser {
    pub fn parse_file(file_name: &str) -> Result<Chart> {
        let content_lines = read_lines(file_name)?;

        let mut general_section = HashMap::new();
        let mut difficulty_section = HashMap::new();
        let mut hit_objects = Vec::new();
        let mut timing_points = Vec::new();
        let mut current_tag = None;

        for line in content_lines {
            let line = line?;

            if line.is_empty() {
                continue;
            }

            if line.starts_with('[') && line.ends_with(']') {
                current_tag = Some(Tag::try_from(line.as_str())?);
                continue;
            }

            if let Some(ref tag) = current_tag {
                match tag {
                    Tag::General => {
                        let parts = line.splitn(2, ':').map(|s| s.trim()).collect::<Vec<_>>();
                        if parts.len() == 2 {
                            general_section.insert(parts[0].to_string(), parts[1].to_string());
                        }
                    }
                    Tag::Difficulty => {
                        let parts = line.splitn(2, ':').map(|s| s.trim()).collect::<Vec<_>>();
                        if parts.len() == 2 {
                            difficulty_section.insert(parts[0].to_string(), parts[1].to_string());
                        }
                    }
                    Tag::HitObjects => {
                        hit_objects.push(Self::parse_hit_object(&line)?);
                    }
                    Tag::TimingPoints => {
                        if let Some(timing_point) = Self::parse_timing_point(&line)? {
                            timing_points.push(timing_point);
                        }
                    }
                    _ => {}
                }
            }
        }

        let audio_file_name = general_section
            .get("AudioFilename")
            .ok_or_else(|| anyhow!("Missing AudioFilename"))?
            .to_string();

        let audio_lead_in: i32 = general_section
            .get("AudioLeadIn")
            .ok_or_else(|| anyhow!("Missing AudioLeadIn"))?
            .parse()
            .map_err(|_| anyhow!("Invalid AudioLeadIn"))?;

        let circle_size: u32 = difficulty_section
            .get("CircleSize")
            .ok_or_else(|| anyhow!("Missing CircleSize"))?
            .parse()
            .map_err(|_| anyhow!("Invalid CircleSize"))?;

        let mode = ChartMode::FixedColumns(circle_size);

        let chart_info = ChartInfo {
            audio_file_name,
            audio_lead_in,
            mode,
        };

        Ok(Chart {
            info: chart_info,
            timing_points: vec![], // Skipping timing points for now
            hit_objects,
            playfield: Playfield { default_speed: 3.0 }, // Assuming Playfield is default constructible
        })
    }

    fn parse_hit_object(line: &str) -> Result<HitObject> {
        let parts = line.split(',').collect::<Vec<_>>();
        if parts.len() < 5 {
            return Err(anyhow!("Malformed HitObject line"));
        }

        let position = (
            parts[0]
                .parse()
                .map_err(|_| anyhow!("Invalid x position"))?,
            parts[1]
                .parse()
                .map_err(|_| anyhow!("Invalid y position"))?,
        );

        let time: f32 = parts[2].parse().map_err(|_| anyhow!("Invalid time"))?;

        let object_type: u32 = parts[3]
            .parse()
            .map_err(|_| anyhow!("Invalid object type"))?;
        let object_parameters = if object_type & 1 != 0 {
            HitObjectParameters::Note
        } else if object_type & 128 != 0 {
            let end_time: f32 = parts[5]
                .split(':')
                .next()
                .ok_or_else(|| anyhow!("Invalid end time"))?
                .parse()
                .map_err(|_| anyhow!("Invalid end time"))?;
            HitObjectParameters::HoldNote(end_time)
        } else {
            return Err(anyhow!("Unsupported object type"));
        };

        Ok(HitObject {
            position,
            time,
            object_parameters,
        })
    }

    fn parse_timing_point(line: &str) -> Result<Option<TimingPoint>> {
        let parts = line.split(',').collect::<Vec<_>>();
        if parts.len() < 8 {
            return Err(anyhow!("Malformed TimingPoint line"));
        }

        let start_time: u32 = parts[0]
            .parse()
            .map_err(|_| anyhow!("Invalid start time"))?;
        let beat_length: f32 = parts[1]
            .parse()
            .map_err(|_| anyhow!("Invalid beat length"))?;
        let meter: u32 = parts[2].parse().map_err(|_| anyhow!("Invalid meter"))?;
        let uninherited: u32 = parts[6]
            .parse()
            .map_err(|_| anyhow!("Invalid uninherited"))?;

        if uninherited == 0 || beat_length < 0.0 {
            return Ok(None);
        }

        Ok(Some(TimingPoint {
            start_time,
            beat_length: TimingPointBeatLength::Duration(beat_length),
            meter,
        }))
    }
}
