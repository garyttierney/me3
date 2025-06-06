# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

<!-- next-header -->

## [Unreleased] - ReleaseDate

### Added

- Clean-up old log files and store them in a per-profile directory by @garyttierney in <https://github.com/garyttierney/me3/pull/84>
- Support overriding files with a game agnostic approach by @Dasaav-dsv in <https://github.com/garyttierney/me3/pull/74>

## [v0.3.0] - 2025-06-02

### Added

- Linux installer via shell script by @garyttierney

### Fixes

- Assign default profile-dir when none has been set by @garyttierney

## [v0.2.0] - 2025-06-01

### Added

- Support for self-updates by running `me3 update` on Windows by @garyttierney
- Add a user friendly command-line interface by @garyttierney in <https://github.com/garyttierney/me3/pull/48>
- Set up documentation site by @garyttierney in <https://github.com/garyttierney/me3/pull/37>
- Support for loading native DLLs by @garyttierney

### Fixes

- Trampoline pointer should be dereferenced by @garyttierney in <https://github.com/garyttierney/me3/pull/39>
- Allow opting out of telemetry by @garyttierney in <https://github.com/garyttierney/me3/pull/61>

### Changes

- Model DLString with cxx-stl and remove cxx dependency by @Dasaav-dsv in <https://github.com/garyttierney/me3/pull/42>
- Add tests for with_context(...) hooks by @garyttierney in <https://github.com/garyttierney/me3/pull/40>
- Normalize profile paths instead of canonicalizing by @garyttierney in <https://github.com/garyttierney/me3/pull/41>
- Structured host->launcher logging by @garyttierney in <https://github.com/garyttierney/me3/pull/43>
- Remove support for YAML ModProfile files by @garyttierney in <https://github.com/garyttierney/me3/pull/50>

## [v0.1.0] - 2025-05-25

### Added

- Loading game assets from local disk by @vswarte in [#22](https://github.com/garyttierney/me3/issues/22)
- Support TOML configuration by @vswarte in [#15](https://github.com/garyttierney/me3/issues/15)
- Crash handling and host<->launcher log transport by @garyttierney in [#24](https://github.com/garyttierney/me3/issues/24)

<!-- next-url -->
[Unreleased]: https://github.com/assert-rs/predicates-rs/compare/v0.3.0...HEAD
[v0.3.0]: https://github.com/assert-rs/predicates-rs/compare/v0.2.0...v0.3.0

[v0.2.0]: https://github.com/assert-rs/predicates-rs/compare/v0.1.0...v0.2.0
