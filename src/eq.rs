// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mateus Rocha de Paula and the OxiGuitar contributors

//! The filters: second-order sections (biquads) with Robert Bristow-Johnson's
//! coefficients ("Cookbook formulae for audio EQ biquad filter
//! coefficients"): low shelf, peak and high shelf, the shelves with slope
//! S = 1. Our own code; no library. Computed in `f64`, run in the
//! transposed direct form II, so even the 31 Hz band stays exact at
//! 96 kHz. No latency.
//!
//! [`Bank`] is what plays: every band the equalizer can have, each gliding
//! to a new gain over [`GLIDE`] seconds when the track's settings change
//! while the song plays (mateusrdp/oxiguitar#232), its coefficients
//! recomputed every [`BLOCK`] frames, so a moved slider or another mode
//! makes no click.

use std::f64::consts::{PI, SQRT_2};

/// A band's shape.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Shape {
    /// Raises or lowers everything below the frequency.
    LowShelf,
    /// Raises or lowers around the frequency; `q` sets how narrowly.
    Peak {
        /// The quality factor: 1.41 is one octave between the points of
        /// half the gain.
        q: f64,
    },
    /// Raises or lowers everything above the frequency.
    HighShelf,
}

/// One band of an equalizer.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Band {
    /// Its shape.
    pub shape: Shape,
    /// Its frequency in Hz: a peak's centre, a shelf's midpoint.
    pub frequency: f64,
    /// Its gain in dB.
    pub gain: f64,
}

/// A band's coefficients at one sample rate, divided by a0.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Coefficients {
    b0: f64,
    b1: f64,
    b2: f64,
    a1: f64,
    a2: f64,
}

/// The highest frequency a band can stand at, as a share of the sample
/// rate: above it the cookbook's filters fold over at half the rate. A
/// band above it (16 kHz at a rate of 32 kHz) is left out.
pub const HIGHEST: f64 = 0.45;

impl Coefficients {
    /// The coefficients of `band` at `rate` Hz, or none when it changes
    /// nothing: a gain of 0 dB, or a frequency the rate can't hold.
    pub fn of(band: Band, rate: f64) -> Option<Coefficients> {
        if band.gain == 0.0
            || !band.gain.is_finite()
            || !(band.frequency > 0.0 && band.frequency < HIGHEST * rate)
        {
            return None;
        }
        let a = 10f64.powf(band.gain / 40.0);
        let w0 = 2.0 * PI * band.frequency / rate;
        let (sin, cos) = w0.sin_cos();
        let (b0, b1, b2, a0, a1, a2) = match band.shape {
            Shape::Peak { q } => {
                let alpha = sin / (2.0 * q);
                (
                    1.0 + alpha * a,
                    -2.0 * cos,
                    1.0 - alpha * a,
                    1.0 + alpha / a,
                    -2.0 * cos,
                    1.0 - alpha / a,
                )
            }
            Shape::LowShelf => {
                // S = 1: alpha = sin(w0)/2 * sqrt(2).
                let k = 2.0 * a.sqrt() * sin / SQRT_2;
                (
                    a * ((a + 1.0) - (a - 1.0) * cos + k),
                    2.0 * a * ((a - 1.0) - (a + 1.0) * cos),
                    a * ((a + 1.0) - (a - 1.0) * cos - k),
                    (a + 1.0) + (a - 1.0) * cos + k,
                    -2.0 * ((a - 1.0) + (a + 1.0) * cos),
                    (a + 1.0) + (a - 1.0) * cos - k,
                )
            }
            Shape::HighShelf => {
                let k = 2.0 * a.sqrt() * sin / SQRT_2;
                (
                    a * ((a + 1.0) + (a - 1.0) * cos + k),
                    -2.0 * a * ((a - 1.0) + (a + 1.0) * cos),
                    a * ((a + 1.0) + (a - 1.0) * cos - k),
                    (a + 1.0) - (a - 1.0) * cos + k,
                    2.0 * ((a - 1.0) - (a + 1.0) * cos),
                    (a + 1.0) - (a - 1.0) * cos - k,
                )
            }
        };
        Some(Coefficients {
            b0: b0 / a0,
            b1: b1 / a0,
            b2: b2 / a0,
            a1: a1 / a0,
            a2: a2 / a0,
        })
    }

    /// The gain in dB at `frequency` Hz, at `rate` Hz: the magnitude of
    /// H(e^jw).
    pub fn response(&self, frequency: f64, rate: f64) -> f64 {
        let w = 2.0 * PI * frequency / rate;
        // H(z) with z^-1 = e^-jw, as (re, im) pairs.
        let (c1, s1) = (w.cos(), -w.sin());
        let (c2, s2) = ((2.0 * w).cos(), -(2.0 * w).sin());
        let num = (
            self.b0 + self.b1 * c1 + self.b2 * c2,
            self.b1 * s1 + self.b2 * s2,
        );
        let den = (
            1.0 + self.a1 * c1 + self.a2 * c2,
            self.a1 * s1 + self.a2 * s2,
        );
        let power = (num.0 * num.0 + num.1 * num.1) / (den.0 * den.0 + den.1 * den.1);
        10.0 * power.log10()
    }
}

/// A band of 0 dB: the sound passes as it is.
const IDENTITY: Coefficients = Coefficients {
    b0: 1.0,
    b1: 0.0,
    b2: 0.0,
    a1: 0.0,
    a2: 0.0,
};

/// One band running on two channels.
#[derive(Clone, Debug)]
struct Stage {
    c: Coefficients,
    /// Each channel's two state values.
    z: [[f64; 2]; 2],
}

impl Stage {
    fn run(&mut self, channel: usize, samples: &mut [f32]) {
        let Coefficients { b0, b1, b2, a1, a2 } = self.c;
        let [z1, z2] = &mut self.z[channel];
        for s in samples {
            let x = f64::from(*s);
            let y = b0 * x + *z1;
            *z1 = b1 * x - a1 * y + *z2;
            *z2 = b2 * x - a2 * y;
            *s = y as f32;
        }
        // A tail decaying in silence would reach the subnormal numbers,
        // which are slow on some processors: below anything audible it
        // stops.
        for z in [z1, z2] {
            if z.abs() < 1e-20 {
                *z = 0.0;
            }
        }
    }
}

/// How long a band takes to glide to a new gain, in seconds.
pub const GLIDE: f64 = 0.02;

/// How many frames a band's coefficients hold while it glides.
pub const BLOCK: usize = 32;

/// Every band an equalizer can have, in series on a stereo track, each
/// gliding to its target gain. A band at 0 dB costs nothing once its
/// last sound has left it.
#[derive(Clone, Debug)]
pub struct Bank {
    rate: f64,
    /// The bands, at the gains they have now.
    bands: Vec<Band>,
    /// The gains they glide to.
    targets: Vec<f64>,
    /// What each gain changes by every block until it is there.
    steps: Vec<f64>,
    stages: Vec<Stage>,
    /// Frames left before the next block.
    left: usize,
    gliding: bool,
}

impl Bank {
    /// `bands`, at their gains, at `rate` Hz. Allocates; the rest
    /// doesn't.
    pub fn new(bands: &[Band], rate: f64) -> Bank {
        Bank {
            rate,
            bands: bands.to_vec(),
            targets: bands.iter().map(|b| b.gain).collect(),
            steps: vec![0.0; bands.len()],
            stages: bands
                .iter()
                .map(|b| Stage {
                    c: Coefficients::of(*b, rate).unwrap_or(IDENTITY),
                    z: [[0.0; 2]; 2],
                })
                .collect(),
            left: 0,
            gliding: false,
        }
    }

    /// Glides to `gains`, one per band in the order of [`Bank::new`]'s;
    /// extra ones are ignored, missing ones keep their target. On the
    /// audio thread: allocates nothing.
    pub fn glide_to(&mut self, gains: &[f64]) {
        let blocks = (GLIDE * self.rate / BLOCK as f64).ceil().max(1.0);
        for (i, &g) in gains.iter().enumerate().take(self.bands.len()) {
            let g = if g.is_finite() { g } else { 0.0 };
            self.targets[i] = g;
            self.steps[i] = (g - self.bands[i].gain) / blocks;
        }
        self.gliding = self
            .bands
            .iter()
            .zip(&self.targets)
            .any(|(b, t)| b.gain != *t);
    }

    /// Whether it changes nothing, now and later.
    pub fn is_flat(&self) -> bool {
        !self.gliding && self.bands.iter().all(|b| b.gain == 0.0)
    }

    /// Its gain in dB at `frequency` Hz, as its bands stand now.
    pub fn response(&self, frequency: f64) -> f64 {
        self.stages
            .iter()
            .map(|s| s.c.response(frequency, self.rate))
            .sum()
    }

    /// The bands' gains now.
    pub fn gains(&self) -> impl Iterator<Item = f64> + '_ {
        self.bands.iter().map(|b| b.gain)
    }

    /// One block's step of every gliding band.
    fn step(&mut self) {
        if !self.gliding {
            return;
        }
        let mut still = false;
        for i in 0..self.bands.len() {
            let (gain, target, step) = (self.bands[i].gain, self.targets[i], self.steps[i]);
            if gain == target {
                continue;
            }
            let next = gain + step;
            let next = if (step > 0.0 && next >= target) || (step <= 0.0 && next <= target) {
                target
            } else {
                still = true;
                next
            };
            self.bands[i].gain = next;
            self.stages[i].c = Coefficients::of(self.bands[i], self.rate).unwrap_or(IDENTITY);
        }
        self.gliding = still;
    }

    /// Filters a track's two channels in place. Allocates nothing.
    pub fn process(&mut self, left: &mut [f32], right: &mut [f32]) {
        let n = left.len().min(right.len());
        let mut at = 0;
        while at < n {
            if self.left == 0 {
                self.step();
                self.left = BLOCK;
            }
            let m = (n - at).min(self.left);
            for stage in &mut self.stages {
                // At 0 dB with nothing left in it, a band is the identity.
                if stage.c == IDENTITY && stage.z == [[0.0; 2]; 2] {
                    continue;
                }
                stage.run(0, &mut left[at..at + m]);
                stage.run(1, &mut right[at..at + m]);
            }
            at += m;
            self.left -= m;
        }
    }
}

/// An equalizer's bands, in series, on a stereo track.
#[derive(Clone, Debug)]
pub struct Chain {
    stages: Vec<Stage>,
}

impl Chain {
    /// The bands `bands` at `rate` Hz; those that change nothing are left
    /// out.
    pub fn new(bands: &[Band], rate: f64) -> Chain {
        Chain {
            stages: bands
                .iter()
                .filter_map(|b| Coefficients::of(*b, rate))
                .map(|c| Stage {
                    c,
                    z: [[0.0; 2]; 2],
                })
                .collect(),
        }
    }

    /// Whether it changes nothing.
    pub fn is_flat(&self) -> bool {
        self.stages.is_empty()
    }

    /// Its gain in dB at `frequency` Hz, at `rate` Hz.
    pub fn response(&self, frequency: f64, rate: f64) -> f64 {
        self.stages
            .iter()
            .map(|s| s.c.response(frequency, rate))
            .sum()
    }

    /// Filters a track's two channels in place. Allocates nothing.
    pub fn process(&mut self, left: &mut [f32], right: &mut [f32]) {
        for stage in &mut self.stages {
            stage.run(0, left);
            stage.run(1, right);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RATES: [f64; 3] = [44_100.0, 48_000.0, 96_000.0];

    fn close(got: f64, wanted: f64, within: f64) {
        assert!(
            (got - wanted).abs() <= within,
            "{got} dB, wanted {wanted} dB within {within}"
        );
    }

    fn peak(frequency: f64, gain: f64, q: f64) -> Band {
        Band {
            shape: Shape::Peak { q },
            frequency,
            gain,
        }
    }

    /// A peak has its full gain at its frequency, and (one octave wide at
    /// Q 1.41, as the cookbook defines its bandwidth) half of it half an
    /// octave either side.
    #[test]
    fn a_peak_is_its_gain_at_its_frequency_and_half_of_it_at_its_edges() {
        for rate in RATES {
            for f in [31.25, 125.0, 700.0, 1000.0, 4000.0, 16_000.0] {
                for gain in [-12.0, -0.5, 0.5, 6.0, 12.0] {
                    let c = Coefficients::of(peak(f, gain, 1.41), rate).unwrap();
                    close(c.response(f, rate), gain, 1e-9);
                    // Far away it changes nothing.
                    close(c.response(f / 64.0, rate), 0.0, 0.05);
                }
            }
            let c = Coefficients::of(peak(1000.0, 12.0, 1.41), rate).unwrap();
            for edge in [1000.0 * 2f64.sqrt(), 1000.0 / 2f64.sqrt()] {
                close(c.response(edge, rate), 6.0, 0.1);
            }
        }
    }

    /// A low shelf has its gain at 0 Hz, half of it at its frequency and
    /// none at half the rate; a high shelf the other way round.
    #[test]
    fn shelves_have_their_gain_on_their_side_and_half_of_it_at_their_frequency() {
        for rate in RATES {
            for gain in [-12.0, -3.5, 4.0, 12.0] {
                let low = Band {
                    shape: Shape::LowShelf,
                    frequency: 120.0,
                    gain,
                };
                let c = Coefficients::of(low, rate).unwrap();
                close(c.response(0.0, rate), gain, 1e-9);
                close(c.response(120.0, rate), gain / 2.0, 1e-9);
                close(c.response(rate / 2.0, rate), 0.0, 1e-9);
                let high = Band {
                    shape: Shape::HighShelf,
                    frequency: 3200.0,
                    gain,
                };
                let c = Coefficients::of(high, rate).unwrap();
                close(c.response(0.0, rate), 0.0, 1e-9);
                close(c.response(3200.0, rate), gain / 2.0, 1e-9);
                close(c.response(rate / 2.0, rate), gain, 1e-9);
            }
        }
    }

    /// A band of 0 dB, or one the rate can't hold, is no band.
    #[test]
    fn a_band_that_changes_nothing_is_left_out() {
        assert_eq!(Coefficients::of(peak(1000.0, 0.0, 1.41), 48_000.0), None);
        assert_eq!(Coefficients::of(peak(16_000.0, 6.0, 1.41), 32_000.0), None);
        assert_eq!(
            Coefficients::of(peak(1000.0, f64::NAN, 1.41), 48_000.0),
            None
        );
        let chain = Chain::new(&[peak(1000.0, 0.0, 1.41)], 48_000.0);
        assert!(chain.is_flat());
        let mut left = [0.25f32, -0.5, 1.0];
        let mut right = left;
        let mut chain = chain;
        chain.process(&mut left, &mut right);
        assert_eq!(left, [0.25, -0.5, 1.0]);
    }

    /// What the chain does to a sine is what its response says: every
    /// band's gain at that frequency, added.
    #[test]
    fn a_sine_through_the_chain_comes_out_at_the_chain_s_response() {
        let rate = 48_000.0;
        let bands = [
            Band {
                shape: Shape::LowShelf,
                frequency: 120.0,
                gain: 6.0,
            },
            peak(700.0, -9.0, 0.8),
            Band {
                shape: Shape::HighShelf,
                frequency: 3200.0,
                gain: 4.5,
            },
        ];
        for f in [60.0, 700.0, 1500.0, 8000.0] {
            let mut chain = Chain::new(&bands, rate);
            let n = 48_000;
            let sine: Vec<f32> = (0..n)
                .map(|i| (2.0 * PI * f * i as f64 / rate).sin() as f32 * 0.25)
                .collect();
            let mut left = sine.clone();
            let mut right: Vec<f32> = sine.iter().map(|s| -s).collect();
            // In pieces, as the player calls it.
            for (l, r) in left.chunks_mut(256).zip(right.chunks_mut(256)) {
                chain.process(l, r);
            }
            // After the first half second, once it has settled.
            let rms = |x: &[f32]| {
                (x.iter().map(|v| f64::from(*v).powi(2)).sum::<f64>() / x.len() as f64).sqrt()
            };
            let measured = 20.0 * (rms(&left[n / 2..]) / rms(&sine[n / 2..])).log10();
            close(measured, chain.response(f, rate), 0.05);
            // The right channel is filtered on its own: the same sine,
            // upside down.
            for (l, r) in left.iter().zip(&right) {
                assert!((l + r).abs() < 1e-6);
            }
        }
    }

    fn bank_bands(gains: [f64; 3]) -> Vec<Band> {
        vec![
            Band {
                shape: Shape::LowShelf,
                frequency: 120.0,
                gain: gains[0],
            },
            peak(700.0, gains[1], 0.8),
            Band {
                shape: Shape::HighShelf,
                frequency: 3200.0,
                gain: gains[2],
            },
        ]
    }

    /// A bank plays as the chain of the same bands does.
    #[test]
    fn a_bank_at_rest_is_its_chain() {
        let bands = bank_bands([6.0, -9.0, 4.5]);
        let sound: Vec<f32> = (0..1000)
            .map(|i| ((i * 37) % 101) as f32 / 101.0 - 0.5)
            .collect();
        let (mut a, mut b) = (sound.clone(), sound.clone());
        let mut chain = Chain::new(&bands, 48_000.0);
        chain.process(&mut a, &mut b);
        let (mut c, mut d) = (sound.clone(), sound.clone());
        let mut bank = Bank::new(&bands, 48_000.0);
        for (l, r) in c.chunks_mut(100).zip(d.chunks_mut(100)) {
            bank.process(l, r);
        }
        for (x, y) in a.iter().zip(&c) {
            assert!((x - y).abs() < 1e-6, "{x} {y}");
        }
        let flat = Bank::new(&bank_bands([0.0; 3]), 48_000.0);
        assert!(flat.is_flat());
        assert!(!bank.is_flat());
    }

    /// #232: a new gain is reached in GLIDE seconds, in small steps: a
    /// steady sine changes level smoothly, never by a jump.
    #[test]
    fn a_new_gain_glides_without_a_jump() {
        let rate = 48_000.0;
        let mut bank = Bank::new(&bank_bands([0.0; 3]), rate);
        let f = 60.0;
        let n = 48_000;
        let mut left: Vec<f32> = (0..n)
            .map(|i| (2.0 * PI * f * i as f64 / rate).sin() as f32 * 0.25)
            .collect();
        let mut right = left.clone();
        let input = left.clone();
        let half = n / 2;
        let (l1, l2) = left.split_at_mut(half);
        let (r1, r2) = right.split_at_mut(half);
        bank.process(l1, r1);
        bank.glide_to(&[12.0, 0.0, 0.0]);
        assert!(!bank.is_flat());
        bank.process(l2, r2);
        assert_eq!(bank.gains().collect::<Vec<_>>(), [12.0, 0.0, 0.0]);
        // Before the change it passes; after it, 12 dB louder.
        assert_eq!(&left[..half], &input[..half]);
        let rms = |x: &[f32]| {
            (x.iter().map(|v| f64::from(*v).powi(2)).sum::<f64>() / x.len() as f64).sqrt()
        };
        let late = &left[n - 4800..];
        let db = 20.0 * (rms(late) / rms(&input[n - 4800..])).log10();
        assert!((db - bank.response(f)).abs() < 0.1, "{db}");
        // No sample jumps further than the loudest sine could move in one
        // frame (a click would).
        let most = 2.0 * PI * f / rate * 0.25 * 10f64.powf(12.0 / 20.0) * 1.5;
        for w in left.windows(2) {
            assert!(
                f64::from((w[1] - w[0]).abs()) <= most,
                "{} to {}",
                w[0],
                w[1]
            );
        }
    }

    /// Back to 0 dB, a band stops costing anything once its sound is out.
    #[test]
    fn a_band_back_at_zero_rests() {
        let mut bank = Bank::new(&bank_bands([6.0, 0.0, 0.0]), 48_000.0);
        let mut l = vec![0.5f32; 4096];
        let mut r = l.clone();
        bank.process(&mut l, &mut r);
        bank.glide_to(&[0.0, 0.0, 0.0]);
        let mut l = vec![0.0f32; 4096];
        let mut r = l.clone();
        bank.process(&mut l, &mut r);
        bank.process(&mut l, &mut r);
        assert!(bank.is_flat());
        assert!(
            bank.stages
                .iter()
                .all(|s| s.c == IDENTITY && s.z == [[0.0; 2]; 2])
        );
    }

    /// Silence after sound ends in exact zeros, never subnormal numbers.
    #[test]
    fn the_tail_ends_in_zeros() {
        let mut chain = Chain::new(&[peak(31.25, 12.0, 1.41)], 96_000.0);
        let mut left = vec![0f32; 4096];
        left[0] = 1.0;
        let mut right = left.clone();
        chain.process(&mut left, &mut right);
        let mut quiet = vec![0f32; 4096];
        let mut quiet_r = quiet.clone();
        for _ in 0..2000 {
            quiet.iter_mut().for_each(|s| *s = 0.0);
            quiet_r.iter_mut().for_each(|s| *s = 0.0);
            chain.process(&mut quiet, &mut quiet_r);
        }
        assert!(chain.stages[0].z.iter().flatten().all(|z| *z == 0.0));
        assert!(quiet.iter().all(|s| *s == 0.0));
    }
}
