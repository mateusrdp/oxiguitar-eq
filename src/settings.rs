// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mateus Rocha de Paula and the OxiGuitar contributors

//! A track's equalizer, kept per track beside the score (`FILE.oxg`, the
//! track dialog's section, under `oxiguitar.eq`):
//!
//! ```json
//! {"mode": "basic", "basic": [0, 0, 0], "graphic": [0, 0, 0, 0, 0, 0, 0, 0, 0, 0]}
//! ```
//!
//! `mode` is `off` (also when missing), `basic` or `graphic`. Both modes
//! keep their values; only the one picked plays, so switching back and
//! forth loses nothing. Every band is −12 to +12 dB in half-decibel steps.

use oxiguitar_plugin::serde_json::{Map, Value, json};

use crate::eq::{Band, Shape};

/// The lowest and highest gain of a band, in dB.
pub const RANGE: (f32, f32) = (-12.0, 12.0);

/// The basic mode's bands, like an amp's tone stack: name, shape and
/// frequency in Hz.
pub const BASIC: [(&str, Shape, f64); 3] = [
    ("Bass", Shape::LowShelf, 120.0),
    ("Middle", Shape::Peak { q: 0.8 }, 700.0),
    ("Treble", Shape::HighShelf, 3200.0),
];

/// The graphic mode's bands, one octave apart from 31 Hz to 16 kHz:
/// name and frequency in Hz (exact octaves of 1 kHz).
pub const GRAPHIC: [(&str, f64); 10] = [
    ("31 Hz", 31.25),
    ("62 Hz", 62.5),
    ("125 Hz", 125.0),
    ("250 Hz", 250.0),
    ("500 Hz", 500.0),
    ("1 kHz", 1000.0),
    ("2 kHz", 2000.0),
    ("4 kHz", 4000.0),
    ("8 kHz", 8000.0),
    ("16 kHz", 16_000.0),
];

/// The graphic mode's bands' Q: one octave between the points of half
/// the gain.
pub const GRAPHIC_Q: f64 = 1.41;

/// Which bands play.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Mode {
    /// None: the track passes untouched.
    #[default]
    Off,
    /// The three bands.
    Basic,
    /// The ten bands.
    Graphic,
}

impl Mode {
    /// The name kept in the settings.
    pub fn key(self) -> &'static str {
        match self {
            Mode::Off => "off",
            Mode::Basic => "basic",
            Mode::Graphic => "graphic",
        }
    }

    fn from_key(key: &str) -> Mode {
        match key {
            "basic" => Mode::Basic,
            "graphic" => Mode::Graphic,
            _ => Mode::Off,
        }
    }
}

/// One track's settings.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Track {
    /// Which bands play.
    pub mode: Mode,
    /// The basic mode's gains in dB, in [`BASIC`]'s order.
    pub basic: [f32; 3],
    /// The graphic mode's gains in dB, in [`GRAPHIC`]'s order.
    pub graphic: [f32; 10],
}

/// A gain as kept: on the half decibel, within [`RANGE`]; 0 for what
/// isn't a number.
pub fn gain(value: f64) -> f32 {
    if !value.is_finite() {
        return 0.0;
    }
    let stepped = ((value * 2.0).round() / 2.0) as f32;
    // No −0: it would be written as -0.0.
    stepped.clamp(RANGE.0, RANGE.1) + 0.0
}

fn gains<const N: usize>(value: Option<&Value>) -> [f32; N] {
    let mut out = [0.0; N];
    if let Some(Value::Array(values)) = value {
        for (o, v) in out.iter_mut().zip(values) {
            *o = v.as_f64().map_or(0.0, gain);
        }
    }
    out
}

impl Track {
    /// Reads a track's settings; anything missing or of the wrong type
    /// keeps its default.
    pub fn from_map(map: &Map<String, Value>) -> Track {
        Track {
            mode: Mode::from_key(map.get("mode").and_then(Value::as_str).unwrap_or("")),
            basic: gains(map.get("basic")),
            graphic: gains(map.get("graphic")),
        }
    }

    /// Reads them from the text the player prepares with.
    pub fn from_text(text: &str) -> Track {
        match oxiguitar_plugin::serde_json::from_str::<Value>(text) {
            Ok(Value::Object(map)) => Track::from_map(&map),
            _ => Track::default(),
        }
    }

    /// Writes them into `map`, keeping keys a later version added.
    pub fn write(&self, map: &mut Map<String, Value>) {
        map.insert("mode".into(), json!(self.mode.key()));
        map.insert("basic".into(), json!(self.basic));
        map.insert("graphic".into(), json!(self.graphic));
    }

    /// The gains of the mode picked: none when it is off.
    pub fn shown(&mut self) -> &mut [f32] {
        match self.mode {
            Mode::Off => &mut [],
            Mode::Basic => &mut self.basic,
            Mode::Graphic => &mut self.graphic,
        }
    }

    /// The bands that play.
    pub fn bands(&self) -> Vec<Band> {
        match self.mode {
            Mode::Off => Vec::new(),
            Mode::Basic => BASIC
                .iter()
                .zip(self.basic)
                .map(|(&(_, shape, frequency), gain)| Band {
                    shape,
                    frequency,
                    gain: f64::from(gain),
                })
                .collect(),
            Mode::Graphic => GRAPHIC
                .iter()
                .zip(self.graphic)
                .map(|(&(_, frequency), gain)| Band {
                    shape: Shape::Peak { q: GRAPHIC_Q },
                    frequency,
                    gain: f64::from(gain),
                })
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_round_trip_and_keep_unknown_keys() {
        let t = Track {
            mode: Mode::Graphic,
            basic: [3.0, -1.5, 0.5],
            graphic: [1.0, 2.0, 3.0, 4.0, 5.0, -6.0, -7.5, 8.0, 12.0, -12.0],
        };
        let mut map = Map::new();
        map.insert("later".into(), json!(1));
        t.write(&mut map);
        assert_eq!(map["mode"], json!("graphic"));
        assert_eq!(map["basic"], json!([3.0, -1.5, 0.5]));
        assert_eq!(map["later"], json!(1));
        assert_eq!(Track::from_map(&map), t);
        assert_eq!(Track::from_text(&Value::Object(map).to_string()), t);
    }

    #[test]
    fn wrong_values_keep_their_defaults_and_gains_keep_to_their_steps() {
        assert_eq!(Track::from_text("not json"), Track::default());
        assert_eq!(Track::from_text("{}").mode, Mode::Off);
        let t = Track::from_text(r#"{"mode": "loud", "basic": [30, -0.3, "x", 7], "graphic": 2}"#);
        assert_eq!(t.mode, Mode::Off);
        // Clamped, on the half decibel, a missing value 0, an extra one
        // left out.
        assert_eq!(t.basic, [12.0, -0.5, 0.0]);
        assert_eq!(t.graphic, [0.0; 10]);
        assert_eq!(gain(0.74), 0.5);
        assert_eq!(gain(0.76), 1.0);
        assert_eq!(gain(-0.1).to_string(), "0");
        assert_eq!(gain(f64::INFINITY), 0.0);
    }

    #[test]
    fn the_gains_shown_are_the_picked_mode_s() {
        let mut t = Track {
            mode: Mode::Basic,
            basic: [1.0, 2.0, 3.0],
            graphic: [4.0; 10],
        };
        assert_eq!(t.shown(), [1.0, 2.0, 3.0]);
        t.mode = Mode::Graphic;
        assert_eq!(t.shown().len(), 10);
        t.mode = Mode::Off;
        assert!(t.shown().is_empty());
    }

    #[test]
    fn only_the_picked_mode_plays() {
        let mut t = Track {
            mode: Mode::Off,
            basic: [6.0, 0.0, -3.0],
            graphic: [1.0; 10],
        };
        assert!(t.bands().is_empty());
        t.mode = Mode::Basic;
        let bands = t.bands();
        assert_eq!(bands.len(), 3);
        assert_eq!(bands[0].shape, Shape::LowShelf);
        assert_eq!((bands[1].frequency, bands[1].gain), (700.0, 0.0));
        assert_eq!(bands[2].gain, -3.0);
        t.mode = Mode::Graphic;
        let bands = t.bands();
        assert_eq!(bands.len(), 10);
        assert_eq!(bands[0].frequency, 31.25);
        assert_eq!(bands[9].frequency, 16_000.0);
        assert!(
            bands
                .iter()
                .all(|b| b.shape == Shape::Peak { q: GRAPHIC_Q })
        );
    }
}
