# Changelog

All notable changes to this project will be documented in this file.

<!-- ignore lint rules that are often triggered by content generated from commits / git-cliff --<!-- markdownlint-disable line-length no-bare-urls ul-style emphasis-style -->
## me3 - [0.5.0](https://github.com/garyttierney/me3/releases/0.5.0) - 2025-06-09

### 🚀 Features

- [d7e8917](https://github.com/garyttierney/me3/commit/d7e891747ed197ed829ffa4a613cccec8f08a68f) Distributed telemetry overhaul in [#113](https://github.com/garyttierney/me3/pull/113)

  > Complete overhaul of the me3 telemetry approach. Now we support
  > distributed spans, capturing backtraces, and associating telemetry with
  > a release version.


### 🐛 Bug Fixes

- [9f574e2](https://github.com/garyttierney/me3/commit/9f574e2399e26230b1b4733a949d920c3884ef7c) CHANGELOG copy-paste error


- [9e5d602](https://github.com/garyttierney/me3/commit/9e5d602a92d2d9a6a8bd6d1cbe7fe2baff22cb3d) Rust formatting


- [0c24d97](https://github.com/garyttierney/me3/commit/0c24d972cfb403046ceb6fca7bd55a9e3912e2ba) Remove openssl


- [1c2699c](https://github.com/garyttierney/me3/commit/1c2699c545fd4a04c1f74092b4421f4f97e69a50) Nightreign file overrides in [#111](https://github.com/garyttierney/me3/pull/111)

  > Extend Nightreign support to all non-wwise files by hooking all
  > overloads of the same function instead of the only one used in earlier
  > games.

  > Get ready to fix missing sound issues when using overrides for
  > Nightreign only by passing down the `me3_mod_protocol::Game` enum. It
  > still needs more work because the ER/AC6 fix does not work, but the
  > infinite loop is still there.

  > Closes #98

- [14c4df0](https://github.com/garyttierney/me3/commit/14c4df029becbadbabe921c937d66683f3579704) Hardcoded app id in steam compat path for proton in [#108](https://github.com/garyttierney/me3/pull/108)


- [96e96e3](https://github.com/garyttierney/me3/commit/96e96e35d237d2d8a85c84baf2f376527e8aa242) Update mod profile schema


- [bfa30cc](https://github.com/garyttierney/me3/commit/bfa30cccd5c4cb7bbfea4808401580ff45b711e7) Update bug report template in release pr


- [05da63c](https://github.com/garyttierney/me3/commit/05da63cfdabe9d0d127a0920c7f63aa07f0a7655) Create asset folder in installer correctly in [#99](https://github.com/garyttierney/me3/pull/99)


- [e1604a4](https://github.com/garyttierney/me3/commit/e1604a4eea2ec0ed01562a7bf9ecae7ca2cc9622) `CrashHandler` being uninstalled via its `Drop` impl in [#91](https://github.com/garyttierney/me3/pull/91)

  > The program-wide crash handler for me3_mod_host.dll was dropped as soon
  > as it was assigned. This fix `mem::forget`s its RAII guard.


### Other

- [9dc14b5](https://github.com/garyttierney/me3/commit/9dc14b5bfa379ed1fedbc618c86f384e41e422fa) Merge remote-tracking branch 'origin/pr-noise' into docs-release-notes-upgrade


- [217b45b](https://github.com/garyttierney/me3/commit/217b45b0da9825a3d818bedddc635455dff3d3b3) Warning when loading NR soundbanks and wems

Temporary hotfix to prevent infinite loops with FIXME comment, I will
address the actual issue soon. in [#112](https://github.com/garyttierney/me3/pull/112)


- [846d446](https://github.com/garyttierney/me3/commit/846d446e967ff8d6e91e4a019422205e4038474a) Warning when loading NR soundbanks and wems


- [ee143c4](https://github.com/garyttierney/me3/commit/ee143c44c764cf3eeda1a04e3a2a867eb0df877c) Update mod profile schema


- [0a0081f](https://github.com/garyttierney/me3/commit/0a0081f50285d4af4f6399c47abb552f301ea923) Add PR link to CHANGELOG.md


- [38a49a9](https://github.com/garyttierney/me3/commit/38a49a9e0116d00cbd757e913fe917638fc78aba) Invert condition


- [0c82fd0](https://github.com/garyttierney/me3/commit/0c82fd03ebf90c9418317812cbc2de6c8a81a621) Add clarifying doc to `Game` enum


- [59e9af8](https://github.com/garyttierney/me3/commit/59e9af8f344ee9a97695fbc546bb50fd058d5c08) Only apply sound workaround in games other than Nightreign


- [104d5c4](https://github.com/garyttierney/me3/commit/104d5c421953563652eae27f877584ec874fa337) Pass down attached game enum


- [596c6d3](https://github.com/garyttierney/me3/commit/596c6d3ec8ba087b2a59b204f4f6bb44eb0e7aca) `CSEblFileManager` is never initialized by this point, so it was a pointless check


- [36da090](https://github.com/garyttierney/me3/commit/36da090cb7a106825eaa6584f1de00c93ef239c1) CHANGELOG.md pre-PR update


- [8de3eba](https://github.com/garyttierney/me3/commit/8de3ebadb49a109042bcc0cb676b87ed3a877274) Hook all 3 `set_path` overloads


- [a0007b7](https://github.com/garyttierney/me3/commit/a0007b73b643e90ffaa47543105ef9df63731514) Add `set_path` overloads


- [46ca634](https://github.com/garyttierney/me3/commit/46ca634b3d39be87f75277bbea2efe6f5919a142) Order games chronologically for `Ord`


- [4c697da](https://github.com/garyttierney/me3/commit/4c697daa0c3252e91dcc16ae744d4d30f74c6a91) Reduce noise from PRs in [#104](https://github.com/garyttierney/me3/pull/104)


- [9a64d3e](https://github.com/garyttierney/me3/commit/9a64d3ec56238c014f81b2f54de8fb6084e0a9eb) Update README.md in [#103](https://github.com/garyttierney/me3/pull/103)


- [b056ecc](https://github.com/garyttierney/me3/commit/b056ecc0e745ecc6301b74bf46c02a8bcc5b496f) Pre-release checks in [#101](https://github.com/garyttierney/me3/pull/101)


- [cbe6c41](https://github.com/garyttierney/me3/commit/cbe6c412acccded4544799d23e4c538495cbd976) Merge branch 'main' into bugfix/installer-assets


- [4bd8b27](https://github.com/garyttierney/me3/commit/4bd8b270d055fb0864c1832324bd2bc66744d417) Cargo fmt


- [72670b7](https://github.com/garyttierney/me3/commit/72670b798df8c4fa2e21b2f67662f2fa7a34184e) Create assets folder in installer


### 📚 Documentation

- [b4047a0](https://github.com/garyttierney/me3/commit/b4047a087941ba68f757e05b34e691142e4b3b58) Add update instructions to release notes in [#114](https://github.com/garyttierney/me3/pull/114)


- [22c909a](https://github.com/garyttierney/me3/commit/22c909a81c6dde6706d807a64ddda0e59d7ab22d) Surround PGP signature in codeblocks


- [80e15f4](https://github.com/garyttierney/me3/commit/80e15f4da82f79b2f9950910d637d0a4b9135251) Update acknowledgements for icon artwork


- [6d3c6c4](https://github.com/garyttierney/me3/commit/6d3c6c4a73122aa5dd2903e9455d4e2014b33fd2) Add RELEASE_CHECKLIST


### ⚙️ Miscellaneous Tasks

- [b0f8a08](https://github.com/garyttierney/me3/commit/b0f8a08e04e53874c24919d8008638b545560338)  *(ci)* Publish pre-releases with version number prefix


- [3177c9b](https://github.com/garyttierney/me3/commit/3177c9b4de67a08224d54dd819c8e630056dce23)  *(ci)* Make sure PDBs are published


- [9b64a8b](https://github.com/garyttierney/me3/commit/9b64a8b9dfc18edfd4d2a0dfebf3a23b86a3075d)  *(ci)* Always upload coverage to codecov

[0.5.0]: https://github.com/garyttierney/me3/compare/v0.4.0..0.5.0

<!-- generated by git-cliff -->
