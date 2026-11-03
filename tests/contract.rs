// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mateus Rocha de Paula and the OxiGuitar contributors

//! The contract suite: builds this crate as an `.oxp` file and checks it
//! through OxiGuitar's own plugin host, as the app would load it, then runs
//! the equalizer through the suite's real-time checks: nothing allocated
//! and (on Linux) no system call made while it runs. `cargo test` runs it
//! with the rest.

use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use oxiguitar_host::{Plugins, SharedSettings, Unheard};

/// This plugin as the `.oxp` file we ship (a debug build), built once.
fn ours() -> &'static Path {
    static BUILT: OnceLock<PathBuf> = OnceLock::new();
    BUILT.get_or_init(|| {
        let crate_folder = Path::new(env!("CARGO_MANIFEST_DIR"));
        // Builds target/oxp/eq.oxp: the package's name, then the file's.
        oxiguitar_plugin_test::build(crate_folder, env!("CARGO_PKG_NAME"), "eq", false)
            .expect("the .oxp file builds")
    })
}

#[test]
fn the_plugin_file_keeps_the_contract() {
    let report = oxiguitar_plugin_test::check(ours());
    for passed in &report.passed {
        println!("ok    {passed}");
    }
    assert!(report.ok(), "failed: {:#?}", report.failed);
}

#[test]
fn the_equalizer_runs_in_real_time() {
    let scratch =
        std::env::temp_dir().join(format!("oxiguitar-eq-realtime-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&scratch);
    std::fs::create_dir_all(&scratch).unwrap();
    std::fs::copy(ours(), scratch.join("eq.oxp")).unwrap();
    let plugins = Plugins::discover(
        std::slice::from_ref(&scratch),
        SharedSettings::default(),
        oxiguitar_host::display_system(),
        Arc::new(Unheard),
    );
    // Every band moved, so every filter runs.
    for settings in [
        r#"{"mode": "basic", "basic": [6, -4.5, 3]}"#,
        r#"{"mode": "graphic", "graphic": [12, -12, 6, -6, 3, -3, 1.5, -1.5, 0.5, -0.5]}"#,
    ] {
        let report = oxiguitar_plugin_test::check_realtime(&plugins, settings);
        assert!(report.ok(), "{settings}: {:#?}", report.failed);
        for wanted in [
            "effect eq: it prepares",
            "effect eq: it runs",
            "effect eq: it allocates nothing",
        ] {
            assert!(
                report.passed.iter().any(|p| p.contains(wanted)),
                "{wanted}: {:#?}",
                report.passed
            );
        }
    }
    let _ = std::fs::remove_dir_all(&scratch);
}
