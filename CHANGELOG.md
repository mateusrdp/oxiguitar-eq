# Changelog

All notable changes to this project are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project
follows [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- The effect `eq` (mateusrdp/oxiguitar#152): an equalizer after a track's
  instrument, for every class of instrument, with or without an amp model.
  Off (the default), **Basic** (Bass, a low shelf at 120 Hz; Middle, a peak
  at 700 Hz, Q 0.8; Treble, a high shelf at 3.2 kHz) or **Graphic** (ten
  peaks one octave apart, 31 Hz to 16 kHz, Q 1.41); every band −12 to
  +12 dB in half-decibel steps. Both modes keep their values; only the one
  picked plays. Kept per track in the song's `.oxg` file under
  `oxiguitar.eq`. Robert Bristow-Johnson's cookbook biquads, our own code;
  no latency.
- `build-oxp.sh`, the contract test and the real-time checks through
  OxiGuitar's plugin host (`tests/contract.rs`), and the release and publish
  workflows (oxiguitar's at v0.12.1).
