// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mateus Rocha de Paula and the OxiGuitar contributors

//! The Track tab's section "Equalizer", for every class of instrument:
//! the mode (Off, Basic, Graphic), a slider per band of the mode picked,
//! and Flat, which sets them to 0 dB. Drawn by `ui/eq.slint` (plugin
//! interface 1.16); this side sets its properties and changes the track's
//! settings when the user moves something.

use std::sync::Mutex;

use oxiguitar_plugin::serde_json::{Map, Value as Json};
use oxiguitar_plugin::{Error, Host, TrackView, Ui, Value};

use crate::SECTION;
use crate::settings::{self, BASIC, GRAPHIC, Mode, RANGE, Track};

/// The modes, in the order the choice lists them.
const MODES: [(Mode, &str); 3] = [
    (Mode::Off, "Off"),
    (Mode::Basic, "Basic (3 bands)"),
    (Mode::Graphic, "Graphic (10 bands)"),
];

/// The names of the bands of `mode`.
fn names(mode: Mode) -> Vec<&'static str> {
    match mode {
        Mode::Off => Vec::new(),
        Mode::Basic => BASIC.iter().map(|b| b.0).collect(),
        Mode::Graphic => GRAPHIC.iter().map(|b| b.0).collect(),
    }
}

/// The section's properties for a track whose settings are `settings`.
pub fn properties(settings: &Map<String, Json>) -> Vec<(String, Value)> {
    let mut t = Track::from_map(settings);
    let mode = t.mode;
    let gains = t.shown().to_vec();
    let bands = names(mode)
        .into_iter()
        .zip(&gains)
        .map(|(name, gain)| {
            Value::Struct(vec![
                ("name".into(), Value::String(name.into())),
                ("gain".into(), Value::Number(f64::from(*gain))),
                ("text".into(), Value::String(format!("{gain:+.1} dB"))),
            ])
        })
        .collect();
    vec![
        (
            "modes".into(),
            Value::List(MODES.iter().map(|m| Value::String(m.1.into())).collect()),
        ),
        (
            "mode".into(),
            Value::Number(MODES.iter().position(|m| m.0 == mode).unwrap_or(0) as f64),
        ),
        ("bands".into(), Value::List(bands)),
        (
            "can-flatten".into(),
            Value::Bool(gains.iter().any(|g| *g != 0.0)),
        ),
        ("minimum".into(), Value::Number(f64::from(RANGE.0))),
        ("maximum".into(), Value::Number(f64::from(RANGE.1))),
    ]
}

fn number(args: &[Value], i: usize) -> Option<f64> {
    match args.get(i) {
        Some(Value::Number(n)) if n.is_finite() => Some(*n),
        _ => None,
    }
}

/// Changes `settings` for the section's callback `callback` with `args`;
/// false when it changes nothing (an unknown callback, a band or mode out
/// of range).
pub fn change(settings: &mut Map<String, Json>, callback: &str, args: &[Value]) -> bool {
    let mut t = Track::from_map(settings);
    match callback {
        "mode-chosen" => match index(args).and_then(|i| MODES.get(i)) {
            Some(m) => t.mode = m.0,
            None => return false,
        },
        "flat" => {
            if t.shown().iter().all(|g| *g == 0.0) {
                return false;
            }
            t.shown().iter_mut().for_each(|g| *g = 0.0);
        }
        "band-slid" => {
            let (Some(i), Some(v)) = (index(args), number(args, 1)) else {
                return false;
            };
            // Only the shown mode's bands exist.
            let Some(band) = t.shown().get_mut(i) else {
                return false;
            };
            *band = settings::gain(v);
        }
        _ => return false,
    }
    t.write(settings);
    true
}

/// The first argument as an index: a whole number from 0.
fn index(args: &[Value]) -> Option<usize> {
    number(args, 0)
        .filter(|i| *i >= 0.0 && i.fract() == 0.0)
        .map(|i| i as usize)
}

/// The section's side in the plugin: the track's settings as the Track
/// tab holds them.
#[derive(Debug)]
pub struct Section {
    host: Host,
    settings: Mutex<Map<String, Json>>,
}

impl Section {
    /// The section, talking to OxiGuitar through `host`.
    pub fn new(host: Host) -> Section {
        Section {
            host,
            settings: Mutex::new(Map::new()),
        }
    }
}

impl Ui for Section {
    fn shown(&self, view: &str, track: Option<&TrackView>) -> Result<(), Error> {
        if view != SECTION {
            return Ok(());
        }
        let settings: Map<String, Json> = track
            .and_then(|t| oxiguitar_plugin::serde_json::from_str(&t.settings).ok())
            .unwrap_or_default();
        let properties = properties(&settings);
        *self.settings.lock().unwrap_or_else(|e| e.into_inner()) = settings;
        self.host.set_view(SECTION, properties)
    }

    fn callback(&self, view: &str, callback: &str, args: Vec<Value>) -> Result<(), Error> {
        if view != SECTION {
            return Ok(());
        }
        let settings = {
            let mut settings = self.settings.lock().unwrap_or_else(|e| e.into_inner());
            if !change(&mut settings, callback, &args) {
                return Ok(());
            }
            settings.clone()
        };
        self.host.set_track_settings(SECTION, &settings)?;
        self.host.set_view(SECTION, properties(&settings))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get<'a>(p: &'a [(String, Value)], name: &str) -> &'a Value {
        &p.iter().find(|(n, _)| n == name).unwrap().1
    }

    fn bands(p: &[(String, Value)]) -> Vec<(String, f64, String)> {
        let Value::List(bands) = get(p, "bands") else {
            panic!()
        };
        bands
            .iter()
            .map(|b| {
                let Value::Struct(f) = b else { panic!() };
                let field = |n: &str| &f.iter().find(|(k, _)| k == n).unwrap().1;
                match (field("name"), field("gain"), field("text")) {
                    (Value::String(n), Value::Number(g), Value::String(t)) => {
                        (n.clone(), *g, t.clone())
                    }
                    other => panic!("{other:?}"),
                }
            })
            .collect()
    }

    fn chose(settings: &mut Map<String, Json>, mode: f64) {
        change(settings, "mode-chosen", &[Value::Number(mode)]);
    }

    fn slid(settings: &mut Map<String, Json>, band: f64, gain: f64) -> bool {
        change(
            settings,
            "band-slid",
            &[Value::Number(band), Value::Number(gain)],
        )
    }

    #[test]
    fn it_shows_the_mode_and_the_picked_mode_s_bands() {
        let mut settings = Map::new();
        // Off: the mode alone.
        assert!(bands(&properties(&settings)).is_empty());
        chose(&mut settings, 1.0);
        let basic = properties(&settings);
        let b = bands(&basic);
        assert_eq!(b.len(), 3);
        assert_eq!(b[0], ("Bass".into(), 0.0, "+0.0 dB".into()));
        assert_eq!(get(&basic, "minimum"), &Value::Number(-12.0));
        // Flat is greyed out while every band is at 0 dB.
        assert_eq!(get(&basic, "can-flatten"), &Value::Bool(false));
        chose(&mut settings, 2.0);
        let graphic = bands(&properties(&settings));
        assert_eq!(graphic.len(), 10);
        assert_eq!(graphic[9].0, "16 kHz");
    }

    /// Each mode keeps its own bands; a band is rounded and clamped; Flat
    /// sets the shown mode's to 0 dB; what is out of range changes
    /// nothing.
    #[test]
    fn the_bands_move_and_flatten() {
        let mut settings = Map::new();
        chose(&mut settings, 1.0);
        assert!(slid(&mut settings, 2.0, 4.3));
        assert!(slid(&mut settings, 0.0, -20.0));
        chose(&mut settings, 2.0);
        assert!(slid(&mut settings, 9.0, -2.74));
        assert!(!slid(&mut settings, 10.0, 1.0));
        chose(&mut settings, 1.0);
        let b = bands(&properties(&settings));
        assert_eq!((b[0].1, b[2].1), (-12.0, f64::from(settings::gain(4.3))));
        assert_eq!(
            get(&properties(&settings), "can-flatten"),
            &Value::Bool(true)
        );
        assert!(change(&mut settings, "flat", &[]));
        assert!(bands(&properties(&settings)).iter().all(|b| b.1 == 0.0));
        assert!(!change(&mut settings, "flat", &[]));
        // The graphic bands were kept.
        chose(&mut settings, 2.0);
        assert_eq!(
            bands(&properties(&settings))[9].1,
            f64::from(settings::gain(-2.74))
        );
        let before = settings.clone();
        assert!(!change(&mut settings, "mode-chosen", &[Value::Number(7.0)]));
        assert!(!change(
            &mut settings,
            "mode-chosen",
            &[Value::Number(-1.0)]
        ));
        assert!(!change(&mut settings, "other", &[]));
        assert_eq!(settings, before);
        assert_eq!(Track::from_map(&settings).mode, Mode::Graphic);
    }

    /// The file compiles as OxiGuitar compiles it, with every property and
    /// callback this side uses.
    #[test]
    fn the_slint_file_has_what_the_section_sets_and_hears() {
        let file = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("ui/eq.slint");
        let compiled = oxiguitar_plugin_test::compile_ui_file(&file).unwrap();
        let c = compiled
            .components
            .iter()
            .find(|c| c.name == "EqSection")
            .unwrap();
        for (name, _) in properties(&Map::new()) {
            assert!(c.properties.contains(&name), "{name}");
        }
        for name in ["mode-chosen", "band-slid", "flat"] {
            assert!(c.callbacks.iter().any(|p| *p == name), "{name}");
        }
    }
}
