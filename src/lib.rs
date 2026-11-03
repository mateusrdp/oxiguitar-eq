// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mateus Rocha de Paula and the OxiGuitar contributors

//! OxiGuitar's equalizer plugin (plugin interface 1.4; mateusrdp/oxiguitar#152,
//! docs/design/clean-and-distorted.md): the effect `eq`, which shapes a
//! track's sound after its instrument, on every class of instrument and
//! with or without an amp model.
//!
//! - `eq` is the filters: Robert Bristow-Johnson's cookbook biquads, our
//!   own code, no latency.
//! - `settings` is a track's choice: off, three bands (Bass, Middle,
//!   Treble) or ten (31 Hz to 16 kHz), each −12 to +12 dB.
//! - `section` shows it in the track dialog, as the section "Equalizer".

use oxiguitar_plugin::audio::PrepareEffect;
use oxiguitar_plugin::{Effect, Error, Host, Plugin, Processor, TrackSection};

pub mod eq;
pub mod section;
pub mod settings;

/// The effect's id in the manifest.
pub const EFFECT: &str = "eq";

/// The track dialog's section's id in the manifest.
pub const SECTION: &str = "eq";

/// The plugin.
#[derive(Debug)]
pub struct EqPlugin;

impl Plugin for EqPlugin {
    fn start(_host: Host) -> Result<EqPlugin, Error> {
        Ok(EqPlugin)
    }

    fn effect(&self, id: &str) -> Option<&dyn Effect> {
        (id == EFFECT).then_some(self as &dyn Effect)
    }

    fn track_section(&self, id: &str) -> Option<&dyn TrackSection> {
        (id == SECTION).then_some(&section::Section as &dyn TrackSection)
    }
}

impl Effect for EqPlugin {
    fn prepare(&self, prepare: &PrepareEffect) -> Result<Box<dyn Processor>, Error> {
        let track = settings::Track::from_text(&prepare.track.settings);
        let chain = eq::Chain::new(&track.bands(), f64::from(prepare.rate));
        Ok(if chain.is_flat() {
            Box::new(Through)
        } else {
            Box::new(Equalizer(chain))
        })
    }
}

/// The equalizer off, or flat: the track's sound passes untouched.
struct Through;

impl Processor for Through {
    fn process(&mut self, _left: &mut [f32], _right: &mut [f32]) {}
}

/// A track through its bands.
struct Equalizer(eq::Chain);

impl Processor for Equalizer {
    fn process(&mut self, left: &mut [f32], right: &mut [f32]) {
        self.0.process(left, right);
    }
}

oxiguitar_plugin::export_plugin!(EqPlugin);

#[cfg(test)]
mod tests {
    use oxiguitar_plugin::audio::PreparedTrack;
    use oxiguitar_plugin::{InstrumentClass, TrackInfo};

    use super::*;

    const MANIFEST: &str = include_str!("../manifest.json");

    #[test]
    fn the_manifest_is_valid_for_the_plugin() {
        let problems = oxiguitar_plugin::check::<EqPlugin>(MANIFEST, env!("CARGO_PKG_VERSION"));
        assert!(problems.is_empty(), "{}", problems.join("; "));
    }

    fn prepared(settings: &str) -> PrepareEffect {
        PrepareEffect {
            effect: EFFECT.into(),
            rate: 48_000,
            frames: 512,
            track: PreparedTrack {
                info: TrackInfo {
                    index: 0,
                    name: "Keys".into(),
                    class: InstrumentClass::Other,
                },
                program: 0,
                percussion: false,
                tuning: Vec::new(),
                capo: 0,
                settings: settings.into(),
            },
        }
    }

    /// Off, flat or with settings it can't read, a track passes untouched;
    /// with a band raised it changes, with no latency.
    #[test]
    fn a_track_changes_only_when_a_band_of_its_mode_is_raised_or_lowered() {
        let sound: Vec<f32> = (0..512)
            .map(|i| ((i * 37) % 101) as f32 / 101.0 - 0.5)
            .collect();
        let run = |settings: &str| {
            let mut p = EqPlugin.prepare(&prepared(settings)).unwrap();
            assert_eq!(p.latency(), 0);
            let (mut l, mut r) = (sound.clone(), sound.clone());
            p.process(&mut l, &mut r);
            (l, r)
        };
        for untouched in [
            "{}",
            "not json",
            r#"{"mode": "off", "basic": [6, 6, 6]}"#,
            r#"{"mode": "basic", "basic": [0, 0, 0], "graphic": [6, 6, 6, 6, 6, 6, 6, 6, 6, 6]}"#,
        ] {
            assert_eq!(
                run(untouched),
                (sound.clone(), sound.clone()),
                "{untouched}"
            );
        }
        let (l, r) = run(r#"{"mode": "graphic", "graphic": [0, 0, 0, 0, 0, 6, 0, 0, 0, 0]}"#);
        assert_ne!(l, sound);
        assert_eq!(l, r);
    }
}
