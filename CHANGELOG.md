# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## Added

- A new `cmdline` feature that builds a `jsonlogic` binary for JsonLogic on
  the commandline

## [0.1.3] - 2020-07-15

- More minor CI fixes

## [0.1.2] - 2020-07-14

### Chore

- A bunch of minor fixes to get the CI pipeline working for all platforms

## [0.1.1] - 2020-07-14

### Fixed
- The Python source dist wasn't generating a Cargo lockfile prior to attempting
  to determine the package version, causing the `cargo pkgid` command to fail

### Chore
- CI fixes for distribution of all the python wheels
- Bumped version to test distribution pipeline

### Docs
- Installation instructions in README

## [0.1.0] - 2020-07-05

### Added
- All standard JSONLogic operations
- WASM build
- Python SDist build
- Packages published & registered on the various package repositories

[Unreleased]: https://github.com/Bestowinc/json-logic-rs/compare/v0.1.2...HEAD
[0.1.2]: https://github.com/Bestowinc/json-logic-rs/compare/v0.1.0...v0.1.2
[0.1.1]: https://github.com/Bestowinc/json-logic-rs/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/Bestowinc/json-logic-rs/compare/0ce0196...v0.1.0
