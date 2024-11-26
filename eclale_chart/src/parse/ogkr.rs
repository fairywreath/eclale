use std::{
    collections::{BTreeMap, HashMap},
    fs,
};

use anyhow::Result;

use ogkr::{
    lex::tokenize,
    parse::{
        analysis::{self as ogkr_analysis, parse_raw_ogkr, Ogkr},
        raw::parse_tokens,
    },
};

use crate::{
    util::{MeasureCompositionData, XPositionCalculator, ZPosition, ZPositionCalculator},
    BpmChange, Chart, ChartData, Composition, ContactNote, ContactNoteType, EvadeNoteType,
    FlickDirection, FlickNote, Header, HitNote, HitNoteType, HoldNote, Lane, LaneType, Metadata,
    Notes, Platform, Soflan, Time, TimeSignature, TimeSignatureChange, Track, TrackPosition,
};

impl From<ogkr_analysis::MeterChange> for TimeSignature {
    fn from(m: ogkr_analysis::MeterChange) -> Self {
        Self {
            num_beats: m.num_beats,
            note_value: m.note_value,
        }
    }
}

impl From<ogkr_analysis::LaneType> for LaneType {
    fn from(t: ogkr_analysis::LaneType) -> Self {
        match t {
            ogkr_analysis::LaneType::Left => LaneType(1),
            ogkr_analysis::LaneType::Center => LaneType(2),
            ogkr_analysis::LaneType::Right => LaneType(3),
            ogkr_analysis::LaneType::Enemy => LaneType(4),
            _ => LaneType(99),
        }
    }
}

impl From<ogkr::lex::command::FlickDirection> for FlickDirection {
    fn from(d: ogkr::lex::command::FlickDirection) -> Self {
        match d {
            ogkr::lex::command::FlickDirection::Left => Self::Left,
            ogkr::lex::command::FlickDirection::Right => Self::Right,
        }
    }
}

struct OgkrChartCreator {
    ogkr: Ogkr,
    z_position_calculator: ZPositionCalculator,
    x_position_calculator: XPositionCalculator,
}

impl OgkrChartCreator {
    fn new(ogkr: Ogkr) -> Self {
        // XXX TODO: Properly handle unwraps or verify this info inside ogkr parser.
        let starting_time_signature = ogkr.header.meter_definition.unwrap();
        let starting_time_signature = TimeSignature {
            num_beats: starting_time_signature.num_beats,
            note_value: starting_time_signature.note_value,
        };
        // XXX TODO FIXME: unwrap bpm def bits inside ogkr parse analysis.
        let starting_bpm = f32::from_bits(ogkr.header.bpm_definition.unwrap().first) as _;
        let starting_speed_multiplier = 1.0;
        let subdivision = ogkr.header.tick_resolution.unwrap().resolution;

        // XXX TODO: Properly query num measures from somewhere.
        let num_measures = 200;

        let z_position_calculator = Self::create_z_position_calculator(
            &ogkr.composition,
            starting_time_signature,
            starting_bpm,
            starting_speed_multiplier,
            subdivision,
            num_measures,
        );

        let x_resolution = ogkr.header.x_resolution.unwrap().resolution;
        let x_position_calculator = XPositionCalculator::new(x_resolution as _);

        Self {
            ogkr,
            z_position_calculator,
            x_position_calculator,
        }
    }

    fn x_position(&self, position: ogkr_analysis::XPosition) -> f32 {
        self.x_position_calculator
            .x_position_at(position.position as _, position.offset as _)
    }

    fn z_position(&self, time: ogkr_analysis::TimingPoint) -> ZPosition {
        self.z_position_calculator
            .z_position_at(time.measure as _, time.beat_offset as _)
    }

    fn create_z_position_calculator(
        composition: &ogkr_analysis::Composition,
        starting_time_signature: TimeSignature,
        starting_bpm: u32,
        starting_speed_multiplier: f32,
        subdivision: u32,
        num_measures: usize,
    ) -> ZPositionCalculator {
        let mut current_bpm_change = composition.bpm_changes.iter();
        let mut current_time_signature_change = composition.meter_changes.iter();
        let mut current_soflan = composition.soflans.iter();

        let mut measure_compositions = Vec::new();
        let mut current_measure = MeasureCompositionData {
            time_signature: starting_time_signature,
            bpm: starting_bpm,
            speed_multiplier: starting_speed_multiplier,
            subdivision,
        };

        for measure_index in 0..num_measures as u32 {
            if let Some((timing_point, change)) = current_bpm_change.clone().next() {
                // -1 because measure starts from 1.
                if measure_index >= timing_point.measure - 1 {
                    current_measure.bpm = change.bpm;
                    current_bpm_change.next();
                }
            }
            if let Some((timing_point, change)) = current_time_signature_change.clone().next() {
                // -1 because measure starts from 1.
                if measure_index >= timing_point.measure - 1 {
                    current_measure.time_signature = change.clone().into();
                    current_time_signature_change.next();
                }
            }
            if let Some((timing_point, soflan)) = current_soflan.clone().next() {
                // -1 because measure starts from 1.
                if measure_index >= timing_point.measure - 1 {
                    current_measure.speed_multiplier = soflan.speed_multiplier;
                    current_soflan.next();
                }
            }

            measure_compositions.push(current_measure.clone());
        }

        // XXX TODO: Properly get audio offset.
        ZPositionCalculator::new(measure_compositions, Time(0.0), 1.0)
    }

    fn create_composition(&self) -> Composition {
        let composition = &self.ogkr.composition;

        let bpm_changes = composition
            .bpm_changes
            .iter()
            .map(|(t, c)| BpmChange {
                time: self.z_position(*t).time,
                bpm: c.bpm,
            })
            .collect();
        let time_signature_changes = composition
            .meter_changes
            .iter()
            .map(|(t, c)| TimeSignatureChange {
                time: self.z_position(*t).time,
                time_signature: c.clone().into(),
            })
            .collect();
        let soflans = composition
            .soflans
            .iter()
            .map(|(t, s)| Soflan {
                time: self.z_position(*t).time,
                // XXX TODO: Properly fill this(in seconds).
                duration: 1.0,
                speed_multiplier: s.speed_multiplier,
            })
            .collect();

        Composition {
            bpm_changes,
            time_signature_changes,
            soflans,
        }
    }

    fn create_track_position(&self, track_position: ogkr_analysis::TrackPosition) -> TrackPosition {
        let z_position = self.z_position(track_position.time);
        TrackPosition {
            time: z_position.time,
            z: z_position.z,
            x: self.x_position(track_position.x),
        }
    }

    fn create_track_positions(
        &self,
        track_positions: &[ogkr_analysis::TrackPosition],
    ) -> Vec<TrackPosition> {
        track_positions
            .iter()
            .map(|p| self.create_track_position(*p))
            .collect()
    }

    fn create_points_from_lane(&self, lane: &ogkr_analysis::Lane) -> Vec<TrackPosition> {
        lane.points
            .iter()
            .map(|p| self.create_track_position(*p))
            .collect()
    }

    fn create_single_lane(
        &self,
        lanes: &BTreeMap<ogkr_analysis::TimingPoint, ogkr_analysis::LaneId>,
    ) -> Lane {
        let points = lanes
            .iter()
            .map(|(_, lane_id)| {
                // XXX TODO: Properly handle unwrap here.
                let lane = self.ogkr.track.get_lane(*lane_id).unwrap();

                self.create_points_from_lane(lane)
            })
            .flatten()
            .collect();

        Lane { points }
    }

    fn create_lanes(
        &self,
        lanes: &BTreeMap<ogkr_analysis::TimingPoint, Vec<ogkr_analysis::LaneId>>,
    ) -> Vec<Lane> {
        lanes
            .iter()
            .map(|(_, lane_ids)| {
                lane_ids
                    .iter()
                    .map(|lane_id| {
                        // XXX TODO: Properly handle unwrap here.
                        let lane = self.ogkr.track.get_lane(*lane_id).unwrap();
                        let points = self.create_points_from_lane(lane);
                        Lane { points }
                    })
                    .collect::<Vec<_>>()
            })
            .flatten()
            .collect()
    }

    fn create_platform(&self) -> Platform {
        let track = &self.ogkr.track;
        let points_left = self.create_single_lane(&track.walls_left).points;
        let points_right = self.create_single_lane(&track.walls_right).points;

        Platform {
            points_left,
            points_right,
        }
    }

    fn create_track(&self) -> Track {
        let platforms = vec![self.create_platform()];

        let mut lanes = HashMap::new();
        let track = &self.ogkr.track;
        lanes.insert(
            ogkr_analysis::LaneType::Left.into(),
            self.create_lanes(&track.lanes_left),
        );
        lanes.insert(
            ogkr_analysis::LaneType::Center.into(),
            self.create_lanes(&track.lanes_center),
        );
        lanes.insert(
            ogkr_analysis::LaneType::Right.into(),
            self.create_lanes(&track.lanes_right),
        );
        lanes.insert(
            ogkr_analysis::LaneType::Enemy.into(),
            self.create_lanes(&track.enemy_lanes),
        );

        Track { platforms, lanes }
    }

    fn hit_note_type(lane: ogkr_analysis::LaneType, is_critical: bool) -> HitNoteType {
        let value = match lane {
            ogkr_analysis::LaneType::WallLeft => 0,
            ogkr_analysis::LaneType::WallRight => 1,
            ogkr_analysis::LaneType::Left => 2,
            ogkr_analysis::LaneType::Center => 3,
            ogkr_analysis::LaneType::Right => 4,
            _ => 99,
        };
        let value = if is_critical { value + 10 } else { value };
        HitNoteType(value)
    }

    fn contact_note_type() -> ContactNoteType {
        ContactNoteType(0)
    }

    fn evade_note_type() -> EvadeNoteType {
        EvadeNoteType(0)
    }

    fn create_hold_notes<'a>(
        &self,
        hold_notes: impl Iterator<Item = &'a ogkr_analysis::HoldNote>,
    ) -> Vec<HoldNote> {
        hold_notes
            .map(|h| HoldNote {
                ty: Self::hit_note_type(h.lane_type, h.is_critical),
                points: h
                    .points
                    .iter()
                    .map(|p| self.create_track_position(*p))
                    .collect(),
            })
            .collect()
    }

    fn create_notes(&self) -> Notes {
        let notes = &self.ogkr.notes;

        let hits = notes
            .all_taps()
            .map(|t| HitNote {
                ty: Self::hit_note_type(t.lane_type, t.is_critical),
                position: self.create_track_position(t.position),
            })
            .collect();

        let contacts = notes
            .all_bells()
            .map(|b| ContactNote {
                ty: Self::contact_note_type(),
                position: self.create_track_position(b.position),
            })
            .collect();
        let flicks = notes
            .all_flicks()
            .map(|f| FlickNote {
                direction: f.direction.into(),
                position: self.create_track_position(f.position),
            })
            .collect();

        let holds = self.create_hold_notes(notes.all_holds());

        // XXX TODO: Fill in from bullets.
        let evades = Vec::new();

        Notes {
            hits,
            holds,
            contacts,
            evades,
            flicks,
        }
    }

    fn create_chart(&self) -> Chart {
        Chart {
            // XXX TODO: Properly fill these.
            header: Header::default(),
            metadata: Metadata::default(),
            data: ChartData {
                track: self.create_track(),
                notes: self.create_notes(),
                composition: self.create_composition(),
            },
        }
    }
}

fn parse_ogkr(file_name: &str) -> Result<Ogkr> {
    let source = fs::read_to_string(file_name)?;
    let tokens = tokenize(&source)?;
    let raw = parse_tokens(tokens)?;

    println!("number of tap tokens {:#?}", raw.notes.taps.len());
    println!("number of flick tokens {:#?}", raw.notes.flicks.len());

    let ogkr = parse_raw_ogkr(raw)?;

    println!("number of ogkr flick notes {:#?}", ogkr.notes.flicks.len());

    Ok(ogkr)
}

pub fn create_chart_from_ogkr_file(file_name: &str) -> Result<Chart> {
    let ogkr = parse_ogkr(file_name)?;

    // println!("{:#?}", &ogkr.notes.taps);
    println!("number of taps {:#?}", ogkr.notes.taps.len());
    let ogkr_chart_creator = OgkrChartCreator::new(ogkr);

    Ok(ogkr_chart_creator.create_chart())
}
