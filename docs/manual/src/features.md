# Features and their tests

Everything the equalizer does, and the tests that check it. The test names are the ones `cargo test` prints. `scripts/test-all.py` fails when a feature here names no test, or names a test that doesn't exist, so a new feature gets its row and its tests in the same pull request (the [Testing](testing.md) page says how to run them).

| Feature | What it does | Tests |
| --- | --- | --- |
| Off by default | A track with the equalizer off passes through untouched, and a band at 0 dB is left out of the chain. | `tests::a_track_changes_only_when_a_band_of_its_mode_is_raised_or_lowered`, `eq::tests::a_band_that_changes_nothing_is_left_out`, `eq::tests::a_band_back_at_zero_rests` |
| Basic mode | Three bands, like an amp's tone stack: Bass (low shelf at 120 Hz), Middle (peak at 700 Hz), Treble (high shelf at 3.2 kHz). | `eq::tests::shelves_have_their_gain_on_their_side_and_half_of_it_at_their_frequency`, `eq::tests::a_peak_is_its_gain_at_its_frequency_and_half_of_it_at_its_edges`, `settings::tests::every_band_is_there_and_only_the_picked_mode_s_have_gains` |
| Graphic mode | Ten peak bands an octave apart, 31 Hz to 16 kHz. | `eq::tests::a_peak_is_its_gain_at_its_frequency_and_half_of_it_at_its_edges`, `eq::tests::a_sine_through_the_chain_comes_out_at_the_chain_s_response`, `settings::tests::every_band_is_there_and_only_the_picked_mode_s_have_gains` |
| Only the picked mode plays | Both modes keep their values; switching loses nothing, and only the picked one is heard and shown. | `settings::tests::only_the_picked_mode_plays`, `settings::tests::the_gains_shown_are_the_picked_mode_s` |
| Gains in half-decibel steps | Each band from -12 to +12 dB in 0.5 dB steps; wrong values in a file fall back to their defaults. | `section::tests::sliders_set_their_band_on_the_half_decibel_and_flat_only_the_shown_mode`, `settings::tests::wrong_values_keep_their_defaults_and_gains_keep_to_their_steps` |
| Flat | Sets the shown mode's bands back to 0 dB, and only that mode's. | `section::tests::sliders_set_their_band_on_the_half_decibel_and_flat_only_the_shown_mode` |
| The Equalizer section of the track dialog | The mode picker and one slider per band of the picked mode. | `section::tests::it_shows_the_mode_and_the_picked_mode_s_bands`, `section::tests::every_mode_s_contents_can_be_shown` |
| Saved with the track | The settings are kept in the score file's plugin data; keys this version doesn't know are kept. | `settings::tests::settings_round_trip_and_keep_unknown_keys` |
| Changes while playing, without clicks | A running equalizer follows new settings; a new gain glides there in 20 ms. | `tests::a_running_equalizer_follows_new_settings`, `eq::tests::a_new_gain_glides_without_a_jump`, `eq::tests::a_bank_at_rest_is_its_chain` |
| A clean tail | When the track stops, the filters ring out to exact zeros. | `eq::tests::the_tail_ends_in_zeros` |
| After the amp | An effect with order 300, so with an amp model on the same track the amp runs first. | `tests::the_manifest_is_valid_for_the_plugin`, `oxiguitar_plugin_checks::the_manifest_matches_the_plugin` |
| Real time | Nothing allocated and no system calls on the audio thread, within its time budget. | `contract::the_equalizer_runs_in_real_time` |
| A working plugin file | The `.oxp` file keeps the plugin contract, built by this repository's SDK version and by the newest release of OxiGuitar. | `contract::the_plugin_file_keeps_the_contract`, `stage:contract (latest core)` |
