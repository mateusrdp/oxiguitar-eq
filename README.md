# oxiguitar-eq

The equalizer plugin for [OxiGuitar](https://github.com/mateusrdp/oxiguitar), a free, open-source guitar tablature and score viewer, editor and player for Windows, macOS and Linux. It shapes a track's sound after its instrument: on any track (guitars, basses, drums, keys), with or without an amp model, so a SoundFont or SFZ track can be shaped where an amp model can't run.

- Plugin file `eq.oxp`, id `oxiguitar.eq`, shown as **Equalizer**.
- Per track, in the track dialog's section **Equalizer** (the Track tab), kept in the song's `.oxg` file beside it:
  - **Off**, the default: the track passes untouched.
  - **Basic**, three bands like an amp's tone stack: Bass (low shelf, 120 Hz), Middle (peak, 700 Hz), Treble (high shelf, 3.2 kHz).
  - **Graphic**, ten bands one octave apart: 31, 62, 125, 250 and 500 Hz, 1, 2, 4, 8 and 16 kHz.
  - Every band −12 to +12 dB in half-decibel steps, one slider each; **Flat** sets the mode's bands back to 0 dB. Both modes keep their values, so switching between them loses nothing.
  - It works while the song plays: the sound changes as you move a slider, each band gliding to its new gain in 20 ms, so nothing clicks.
- No latency, and nothing allocated while it plays. The filters are Robert Bristow-Johnson's cookbook biquads, our own code, with no other library.
- With the NAM plugin on the same track, OxiGuitar runs the amp first and then the equalizer; the track dialog moves one before the other.

## Manual

The manual is at <https://mateusrdp.github.io/oxiguitar-eq/>. Its source is in [docs/manual](docs/manual). It covers what the plugin is for, how it works, what it needs and how to install it.

## Install

Every [release](https://github.com/mateusrdp/oxiguitar-eq/releases) holds a zip per system (Linux x86_64 and ARM64, Windows x86_64 and ARM64, macOS), a `.deb` and an `.rpm` for Linux, a per-user Windows installer (no administrator needed), a macOS `.pkg`, and a `SHA256SUMS` file. The [manual's install page](https://mateusrdp.github.io/oxiguitar-eq/install.html) says which one to use and how to keep it updated. The plugin needs OxiGuitar 0.20.0 or later (plugin interface 1.8); its 0.2 releases work with 0.14 and later, its 0.1 releases with 0.11 and later.

## Build and test

Rust 1.99 or later from [rustup](https://rustup.rs). The plugin SDK comes from OxiGuitar's repository by git tag (`v0.14.0`); nothing of OxiGuitar's is on crates.io.

```sh
cargo test          # unit tests (the filters against the cookbook's responses), the contract suite on the built .oxp file, and the real-time checks
./build-oxp.sh      # dist/eq-VERSION-SYSTEM-ARCH.oxp
python3 scripts/test-all.py   # every test, as the manual's Testing page says
```

## Licence

GPL-3.0-or-later. See LICENSE. Contributions are welcome under the same terms; see [CONTRIBUTING.md](CONTRIBUTING.md).
