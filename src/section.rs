// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mateus Rocha de Paula and the OxiGuitar contributors

//! The track dialog's section "Equalizer", for every class of instrument:
//! the mode (Off, Basic, Graphic), a slider per band of the mode picked,
//! and Flat, which sets them to 0 dB. A drawn curve would need a canvas,
//! which track sections don't have (interface 1.4), so the bands are
//! sliders, as the NAM section's levels are.

use oxiguitar_plugin::serde_json::{Map, Value};
use oxiguitar_plugin::{Error, Row, TabContent, TabEvent, TrackInfo, TrackSection, Widget};

use crate::settings::{self, BASIC, GRAPHIC, Mode, RANGE, Track};

/// The modes, in the order the choice lists them.
const MODES: [(Mode, &str); 3] = [
    (Mode::Off, "Off"),
    (Mode::Basic, "Basic (3 bands)"),
    (Mode::Graphic, "Graphic (10 bands)"),
];

/// The id of the Flat button.
pub const FLAT: &str = "flat";

/// The section.
#[derive(Debug, Default)]
pub struct Section;

fn label(text: impl Into<String>, muted: bool) -> Widget {
    Widget::Label {
        text: text.into(),
        muted,
    }
}

/// A band's slider's id: the mode's key, then the band's place
/// (`basic:0`, `graphic:9`).
fn band_id(mode: Mode, index: usize) -> String {
    format!("{}:{index}", mode.key())
}

/// The names of the bands of `mode`.
fn names(mode: Mode) -> Vec<&'static str> {
    match mode {
        Mode::Off => Vec::new(),
        Mode::Basic => BASIC.iter().map(|b| b.0).collect(),
        Mode::Graphic => GRAPHIC.iter().map(|b| b.0).collect(),
    }
}

impl TrackSection for Section {
    fn content(&self, _track: &TrackInfo, settings: &Map<String, Value>) -> TabContent {
        let mut t = Track::from_map(settings);
        let mode = t.mode;
        let mut rows = vec![Row {
            widgets: vec![
                label("Mode", false),
                Widget::Choice {
                    id: "mode".into(),
                    options: MODES.iter().map(|m| m.1.to_owned()).collect(),
                    selected: MODES.iter().position(|m| m.0 == mode).unwrap_or(0) as u32,
                    enabled: true,
                },
            ],
        }];
        let gains = t.shown();
        for (i, (name, gain)) in names(mode).into_iter().zip(gains.iter()).enumerate() {
            rows.push(Row {
                widgets: vec![
                    label(name, false),
                    Widget::Slider {
                        id: band_id(mode, i),
                        value: *gain,
                        min: RANGE.0,
                        max: RANGE.1,
                        enabled: true,
                    },
                    label(format!("{gain:+.1} dB"), true),
                ],
            });
        }
        if mode != Mode::Off {
            rows.push(Row {
                widgets: vec![Widget::Button {
                    id: FLAT.into(),
                    text: "Flat".into(),
                    enabled: gains.iter().any(|g| *g != 0.0),
                }],
            });
        }
        TabContent { rows }
    }

    fn on(
        &self,
        _track: &TrackInfo,
        settings: &mut Map<String, Value>,
        widget: &str,
        event: TabEvent,
    ) -> Result<(), Error> {
        let mut t = Track::from_map(settings);
        match (widget, event) {
            ("mode", TabEvent::Chosen(i)) => match MODES.get(i as usize) {
                Some(m) => t.mode = m.0,
                None => return Ok(()),
            },
            (FLAT, TabEvent::Clicked) => t.shown().iter_mut().for_each(|g| *g = 0.0),
            (id, TabEvent::Slid(v)) => {
                // Only the shown mode's sliders exist; one of another mode
                // (an answer to a dialog since changed) changes nothing.
                let Some(i) = (0..t.shown().len()).find(|&i| band_id(t.mode, i) == id) else {
                    return Ok(());
                };
                t.shown()[i] = settings::gain(f64::from(v));
            }
            _ => return Ok(()),
        }
        t.write(settings);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use oxiguitar_plugin::InstrumentClass;
    use oxiguitar_plugin::serde_json::json;

    use super::*;

    fn track() -> TrackInfo {
        TrackInfo {
            index: 0,
            name: "Drums".into(),
            class: InstrumentClass::Drums,
        }
    }

    fn ids(content: &TabContent) -> Vec<String> {
        content
            .rows
            .iter()
            .flat_map(|r| &r.widgets)
            .filter_map(|w| match w {
                Widget::Choice { id, .. }
                | Widget::Slider { id, .. }
                | Widget::Button { id, .. } => Some(id.clone()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn it_shows_the_mode_and_the_picked_mode_s_bands() {
        let mut settings = Map::new();
        // Off: the mode alone.
        assert_eq!(ids(&Section.content(&track(), &settings)), ["mode"]);
        Section
            .on(&track(), &mut settings, "mode", TabEvent::Chosen(1))
            .unwrap();
        let basic = Section.content(&track(), &settings);
        assert_eq!(ids(&basic), ["mode", "basic:0", "basic:1", "basic:2", FLAT]);
        assert!(matches!(
            &basic.rows[1].widgets[..],
            [Widget::Label { text, .. }, Widget::Slider { min: -12.0, max: 12.0, .. }, Widget::Label { text: value, muted: true }]
                if text == "Bass" && value == "+0.0 dB"
        ));
        // Flat is greyed out while every band is at 0 dB.
        assert!(matches!(
            basic.rows[4].widgets[0],
            Widget::Button { enabled: false, .. }
        ));
        Section
            .on(&track(), &mut settings, "mode", TabEvent::Chosen(2))
            .unwrap();
        let graphic = Section.content(&track(), &settings);
        assert_eq!(ids(&graphic).len(), 1 + 10 + 1);
        assert!(matches!(
            &graphic.rows[10].widgets[0],
            Widget::Label { text, .. } if text == "16 kHz"
        ));
    }

    #[test]
    fn sliders_set_their_band_on_the_half_decibel_and_flat_only_the_shown_mode() {
        let mut settings = Map::new();
        settings.insert("later".into(), json!(true));
        let on = |s: &mut Map<String, Value>, id: &str, e: TabEvent| {
            Section.on(&track(), s, id, e).unwrap();
        };
        on(&mut settings, "mode", TabEvent::Chosen(1));
        on(&mut settings, "basic:2", TabEvent::Slid(4.3));
        on(&mut settings, "basic:0", TabEvent::Slid(-20.0));
        // A slider of the mode not shown changes nothing.
        on(&mut settings, "graphic:3", TabEvent::Slid(5.0));
        on(&mut settings, "mode", TabEvent::Chosen(2));
        on(&mut settings, "graphic:9", TabEvent::Slid(-2.74));
        on(&mut settings, "basic:1", TabEvent::Slid(5.0));
        let t = Track::from_map(&settings);
        assert_eq!(t.mode, Mode::Graphic);
        assert_eq!(t.basic, [-12.0, 0.0, 4.5]);
        assert_eq!(
            t.graphic,
            [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, -2.5]
        );
        let content = Section.content(&track(), &settings);
        assert!(matches!(
            content.rows[11].widgets[0],
            Widget::Button { enabled: true, .. }
        ));
        // Flat sets the graphic bands to 0 dB; the basic ones stay, and
        // come back with their mode.
        on(&mut settings, FLAT, TabEvent::Clicked);
        let t = Track::from_map(&settings);
        assert_eq!(t.graphic, [0.0; 10]);
        assert_eq!(t.basic, [-12.0, 0.0, 4.5]);
        on(&mut settings, "mode", TabEvent::Chosen(1));
        assert_eq!(Track::from_map(&settings).mode, Mode::Basic);
        // A line the choice doesn't have, or a widget it doesn't know,
        // changes nothing; keys of a later version stay.
        let before = settings.clone();
        on(&mut settings, "mode", TabEvent::Chosen(7));
        on(&mut settings, "other", TabEvent::Clicked);
        assert_eq!(settings, before);
        assert_eq!(settings["later"], json!(true));
    }
}
