# Architecture and strategy

## How it fits into OxiGuitar

The plugin is an *effect*: it sits on a track after the instrument that plays it. OxiGuitar hands it the track's sound in small blocks while the song plays, and the plugin hands them back changed. With the NAM plugin on the same track, the amp runs first and then the equalizer, as on a real rig where the EQ comes after the amp; the arrows in the track dialog swap them.

Each band is a *biquad* filter, the standard small building block of digital EQs, designed from Robert Bristow-Johnson's *Audio EQ Cookbook*: low shelf, peak and high shelf for Basic, peaks for Graphic. The code is ours, with no other library. It runs on the audio thread with no latency and no memory allocated, and each band glides to a new gain over 20 ms so moving a slider never clicks.

The settings are saved per track in the song's `.oxg` file. The score itself doesn't change.

## Current limitations

- **Fixed frequencies.** You set how much, not where: Basic's three frequencies and Graphic's ten are fixed, and so is how wide each band is.
- **No analyser.** There is no picture of the track's sound to aim at. You set it by ear.
- **No presets** to save and reuse across songs.
- **±12 dB per band.** Enough for tone shaping, not for surgery.

## Left out on purpose

- **A parametric EQ.** Freely movable frequencies and widths are powerful and easy to get lost in. The plugin aims to be simple and predictable: one slider per band and nothing hidden. A parametric mode could still come later as its own mode.
- **Compression, limiting and the like.** Those are other effects and belong in other plugins.
- **Other libraries.** The filters are a few dozen lines of well-known maths. Writing them ourselves keeps the plugin small and lets the tests compare every filter against the cookbook's own formulas.

## Where you can help

- **Presets**: a small list of starting points (a "scooped" metal tone, a brighter acoustic) would help beginners.
- **A response curve** drawn in the section, like the picture in the [overview](index.html), so you can see what the sliders do.
- **Listening**: tell us if a band feels too wide or too narrow on real songs.

Every change needs its tests and a row on the [Features](features.md) page; the [Testing](testing.md) page says how to run them.
