// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mateus Rocha de Paula and the OxiGuitar contributors

//! OxiGuitar's equalizer plugin (plugin interface 1.5; mateusrdp/oxiguitar#152,
//! docs/design/clean-and-distorted.md): the effect `eq`, which shapes a
//! track's sound after its instrument, on every class of instrument and
//! with or without an amp model.
//!
//! - `eq` is the filters: Robert Bristow-Johnson's cookbook biquads, our
//!   own code, no latency.
//! - `settings` is a track's choice: off, three bands (Bass, Middle,
//!   Treble) or ten (31 Hz to 16 kHz), each −12 to +12 dB.
//! - `section` shows it in the track dialog, as the section "Equalizer".
//!
//! A change of the section while the song plays reaches the running
//! equalizer (interface 1.5), whose bands glide to their new gains.

use oxiguitar_plugin::audio::PrepareEffect;
use oxiguitar_plugin::{Effect, Error, Host, Plugin, Processor, TrackSection, Update};

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
        Ok(Box::new(Equalizer(eq::Bank::new(
            &track.every_band(),
            f64::from(prepare.rate),
        ))))
    }

    /// The track's new settings while the song plays (interface 1.5,
    /// mateusrdp/oxiguitar#232): every band's new gain, which the bands
    /// glide to.
    fn update(&self, prepare: &PrepareEffect, _previous: &str) -> Result<Option<Update>, Error> {
        let track = settings::Track::from_text(&prepare.track.settings);
        let gains: [f64; settings::ALL] = track.every_band().map(|b| b.gain);
        Ok(Some(Update::new(gains)))
    }
}

/// A track through every band. A band at 0 dB costs nothing, so off or
/// flat the sound passes untouched, and it can still glide into a new
/// setting while the song plays.
struct Equalizer(eq::Bank);

impl Processor for Equalizer {
    fn process(&mut self, left: &mut [f32], right: &mut [f32]) {
        self.0.process(left, right);
    }

    fn apply(&mut self, update: &mut Update) {
        if let Some(gains) = update.get::<[f64; settings::ALL]>() {
            self.0.glide_to(gains);
        }
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

    /// A change while the song plays: the running equalizer takes the new
    /// settings and glides to them; settings it can't read are flat.
    #[test]
    fn a_running_equalizer_follows_new_settings() {
        let raised = r#"{"mode": "basic", "basic": [12, 0, 0]}"#;
        let mut p = EqPlugin.prepare(&prepared("{}")).unwrap();
        let sound = vec![0.25f32; 4800];
        let run = |p: &mut Box<dyn Processor>| {
            let (mut l, mut r) = (sound.clone(), sound.clone());
            for (a, b) in l.chunks_mut(480).zip(r.chunks_mut(480)) {
                p.process(a, b);
            }
            l
        };
        assert_eq!(run(&mut p), sound);
        let mut u = EqPlugin.update(&prepared(raised), "{}").unwrap().unwrap();
        p.apply(&mut u);
        let louder = run(&mut p);
        // A steady signal, a low shelf of 12 dB: about four times louder
        // once it has glided there, and on its way within the first block.
        assert!(
            (louder[4799] / 0.25 - 10f32.powf(0.6)).abs() < 0.02,
            "{}",
            louder[4799]
        );
        assert!(louder[0] < 0.26, "{}", louder[0]);
        let mut u = EqPlugin
            .update(&prepared("not json"), "{}")
            .unwrap()
            .unwrap();
        p.apply(&mut u);
        run(&mut p);
        let back = run(&mut p);
        assert!((back[4799] - 0.25).abs() < 1e-4, "{}", back[4799]);
    }
}
