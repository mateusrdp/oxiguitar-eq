# Testing

Every test of this repository runs with one command, on your own machine, and anyone can run it. When something fails, it tells you how to report it.

## Run everything

```sh
python3 scripts/test-all.py
```

On Windows, type `python` instead of `python3` if that's what you have.

You need:

- Rust, from [rustup](https://rustup.rs) (the version in `Cargo.toml`'s `rust-version` or newer);
- git, and Python 3.8 or newer (nothing to install with pip);
- on Linux, the libraries a build needs: `libasound2-dev`, `libpipewire-0.3-dev` and `libclang-dev` on Debian and Ubuntu (`alsa-lib-devel`, `pipewire-devel` and `clang-devel` on Fedora);

It runs:

1. every Rust test of the repository (`cargo test` over the whole workspace), with a progress bar;
1. the plugin against the **newest release of OxiGuitar**: it fetches that release's source with git (once per release, kept in a cache folder every OxiGuitar repository on your machine shares: `~/.cache/oxiguitar-test-all`, `~/Library/Caches/oxiguitar-test-all` on macOS, `%LOCALAPPDATA%\oxiguitar-test-all` on Windows, or `OXIGUITAR_TEST_CACHE`), builds its `oxiguitar-cli` and `oxiguitar-plugin-test`, builds this repository's plugin file for your system and runs the plugin contract on it;
1. a check of the [Features](features.md) page: every feature names a test, and every test it names exists.

It tests the system and processor it runs on, and only those (as Rust sees them: `rustc -vV`'s host). Tests for other systems are left out and the summary says so; they are not failures.

## What you get

A line per stage while it runs, then the result. In `target/test-all/<date>-<time>/`:

- `summary.md`: the result, your system, what failed and why, short enough to paste into a GitHub issue;
- `test-all.log`: everything that happened, every command and its output;

The script exits with 0 when everything passed, 1 when something failed, and 2 when it couldn't start.

## When something fails

1. Update to the newest `develop` (`git checkout develop && git pull`) and run it again: it may be fixed already.
2. Look for an issue about the same test at [github.com/mateusrdp/oxiguitar-eq/issues](https://github.com/mateusrdp/oxiguitar-eq/issues).
3. Otherwise [open one](https://github.com/mateusrdp/oxiguitar-eq/issues/new): call it `test-all: <the first failing test>`, paste the whole `summary.md`, and attach `test-all.log` (drag it onto the text box; zip it first if it's over 25 MB). Read the log first: it has your folder names, so change anything you'd rather keep private.
4. You can also give `summary.md` and `test-all.log` to a language model along with this repository and ask it to find the cause.

## Options

- `--release`: an optimised test build: slower to build, faster corpus tests.
- `--ignore-rust-version`: for a Rust older than `rust-version` (unsupported, but handy).
- `--core DIR`: test against a checkout of OxiGuitar you already have, instead of its newest release; `--core-version v0.21.5` picks a release.
- `--out DIR`: where the results go.

## Adding a feature

A new feature comes with its tests and a row on the [Features](features.md) page, in the same pull request; a new corpus test uses `tests/support/corpus.rs`. The script fails until the row names tests that exist.
