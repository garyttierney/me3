# Changelog

All notable changes to this project will be documented in this file.
<!-- markdown-link-check-disable --<!-- ignore lint rules that are often triggered by content generated from commits / git-cliff --<!-- markdownlint-disable line-length no-bare-urls ul-style emphasis-style -->
## me3 - [v0.5.0](https://github.com/garyttierney/me3/releases/v0.5.0) - 2025-06-14

### 🚀 Features

- [0b79c75](https://github.com/garyttierney/me3/commit/0b79c75539318d99b6fb13c39119c79d84b317c2) Suspended attach in [#142](https://github.com/garyttierney/me3/pull/142)
  > me3_launcher now attaches and executes the `me_attach` me3_mod_host
  > entrypoint in a suspended process without polling for kernel32.dll.
  > Added a `me3 launch --suspend` flag to only execute the entrypoint after
  > a debugger is attached to the process.


- [bf7f4c7](https://github.com/garyttierney/me3/commit/bf7f4c73272daf0aea54b415b653fd067956ea4d) Support for `--suspend` launch flag



- [ef5084d](https://github.com/garyttierney/me3/commit/ef5084d272b1a063b2c05d74d08eb023ffe747ba) Defer applying asset override hooks



- [e6483e9](https://github.com/garyttierney/me3/commit/e6483e9480f60b31d8df6b80364d6187adb03908) Suspend main thread on attach



- [ef685e7](https://github.com/garyttierney/me3/commit/ef685e7c6767fcdae4950619781143d8257bad09) Improvements to out of the box UX in [#122](https://github.com/garyttierney/me3/pull/122)
  > Adds start menu entries and a 'mods' folder for each profile where mods can be placed for an out-of-the box launch experience.


- [124b11d](https://github.com/garyttierney/me3/commit/124b11d525f5170a4501ba885be56774bc90351e) Make any prompts shown by CLI DPI aware



- [59b4954](https://github.com/garyttierney/me3/commit/59b4954403c2852c964f91f5ce68cd53772e2073) Create start menu entry to documentation



- [a280a92](https://github.com/garyttierney/me3/commit/a280a92dd6a88e30b50793de6de3857d15b55d24) Create start-menu shortcuts for default profiles



- [cc6b231](https://github.com/garyttierney/me3/commit/cc6b2311587759a98732f6bcf76817de4dff2f13) Create drop-in mod folders for default profiles



- [1a8bde1](https://github.com/garyttierney/me3/commit/1a8bde1caf10f8b4dec3dd8473cfc697740be70b) Support loading mods that rely on modengine_ext_init



### 🐛 Bug Fixes

- [0c68dae](https://github.com/garyttierney/me3/commit/0c68daec648902a3dfa4574b6e5e80f26c703c34)  *(linux)* Fallback to global steam compat tool in [#134](https://github.com/garyttierney/me3/pull/134)



- [98b7998](https://github.com/garyttierney/me3/commit/98b7998532b977bea0828632b15e92880054cb62)  *(linux)* Ensure config home exists before creating default config in [#126](https://github.com/garyttierney/me3/pull/126)


  > Fixes #125

- [2e4af9b](https://github.com/garyttierney/me3/commit/2e4af9b96092de68b8bbcb6bc8d31b40b717bd82)  *(overrides)* UXM disk overrides in [#156](https://github.com/garyttierney/me3/pull/156)
  > Normalize all paths with `normpath` and better handle OS-encoded
  > strings.

  > This addresses an issue where me3 could not provide overrides for a game
  > patched with UXM, where a relative path like
  > `".//////sound/soundbanksinfo.mobnkinfo"` would not be overidden.

  > Tested on ERR and UXM-patched NR.


- [b6ade25](https://github.com/garyttierney/me3/commit/b6ade25a9141ca139f1308c7d30fc092473c8e7f)  *(overrides)* Normalize paths and clean up unnecessary branching



- [c166053](https://github.com/garyttierney/me3/commit/c166053582fcf6ab352b5d5d044ea07e179bc732) Creation of release PR



- [c8a98d5](https://github.com/garyttierney/me3/commit/c8a98d589fbd8b0832f9844f25c0b0930d5594b3) Logging for applied hooks



- [757f998](https://github.com/garyttierney/me3/commit/757f9987a0ca2c5b3f1c78ff03382c277fff880a) Support native debuggers on WINE



- [f36fb05](https://github.com/garyttierney/me3/commit/f36fb055a25c72fa67f8e663fe0b40ce94089712) Reduce console log pollution



- [1e7583c](https://github.com/garyttierney/me3/commit/1e7583c450c820277148c601cdfc561549ddc4f6) Ensure me3-telemetry can be compiled without sentry in [#143](https://github.com/garyttierney/me3/pull/143)



- [c9a94fb](https://github.com/garyttierney/me3/commit/c9a94fb4e5407669bc7c0ace529bd5d9f61df202) Error reporting for elevation errors in [#138](https://github.com/garyttierney/me3/pull/138)
  > i.e. when "Run as Administrator" compatability setting is used


- [06fa12f](https://github.com/garyttierney/me3/commit/06fa12f1984249a2de559cc4b6f8dbc0efeda6f0) Missing instrument



- [b5fbac7](https://github.com/garyttierney/me3/commit/b5fbac7fe331acd4f8f370ef5d1acc3fedf7879c) Error reporting when elevation would be needed



- [ca87788](https://github.com/garyttierney/me3/commit/ca87788527465625fc13f1f58e8fc5cac19eabda) Require Steam to be running before launching the game



- [3d58baf](https://github.com/garyttierney/me3/commit/3d58baf6a3c242f0d1434de4deddeeee663f5237) Wwise overrides for ER/AC6/NR in [#128](https://github.com/garyttierney/me3/pull/128)
  > Replaces export polling with scanning for another injection point to
  > avoid the infinite loop bug encountered in fs code. This allows for
  > proper soundbank and wem overrides in NR, and removes the need for the
  > workaround in ER/AC6 that reported any new bnds as having failed to
  > mount.


- [4b2d18e](https://github.com/garyttierney/me3/commit/4b2d18e663dc057ad2b437f429e54a7c038b9eb6) Skip(n).next() -> nth(n)



- [ce55a3f](https://github.com/garyttierney/me3/commit/ce55a3f24d240f4ec50deb1a34726f775bc2bd32) Update test expectations for renamed fields



- [efa5091](https://github.com/garyttierney/me3/commit/efa50916daf1f32011583cbf1c85d3ed5bd5d48c) Include start menu entries during uninstall



- [3767555](https://github.com/garyttierney/me3/commit/3767555cdf1d041cfebd354a21e666ec9942a9d9) Handle attach errors before waiting on game shutdown



- [3834a44](https://github.com/garyttierney/me3/commit/3834a44d33633fd3c1b0df47c695f3c175172cab) Remove skip_serializing_if due to bincode bug



- [a00d126](https://github.com/garyttierney/me3/commit/a00d12684e43ba03bf7eefd4c7de68884edb3237) Path to me3 host DLL during uninstall



- [dfae8ab](https://github.com/garyttierney/me3/commit/dfae8aba8b55cd4a98154ab5aa764bdc1f35a9e6) Switch non-canonical names to aliases



- [acf0fd2](https://github.com/garyttierney/me3/commit/acf0fd213aa948e30a0b7bd41e54c515e21dab6b) Misc. fixes for Linux CLI in [#116](https://github.com/garyttierney/me3/pull/116)
  > - Resolve the Proton prefix from the library the game is actually
  > installed in.
  > - Install eyre panic/error hook earlier so auto-install doesn't get
  > triggered by a missing system profile dir.
  > - Support Seamless Coop


- [39a873f](https://github.com/garyttierney/me3/commit/39a873f19ce6d316f3b4cd1ec9a813485d3c0618) Resolve proton prefix from steam library game is installed in



- [35ed61a](https://github.com/garyttierney/me3/commit/35ed61a3946ceb90999f7ad4caf7dfaf4818aa04) Install error handler during startup



- [e3ed588](https://github.com/garyttierney/me3/commit/e3ed588ed1d49f06bf86d3310ed66fea6fb86e54) Prevent eyre error handles from being auto-installed
  > The 'auto-install' feature of eyre is responsible for this, so get rid
  > of default features and use what we need.


- [9f574e2](https://github.com/garyttierney/me3/commit/9f574e2399e26230b1b4733a949d920c3884ef7c) CHANGELOG copy-paste error



### Other

- [3c1a4b0](https://github.com/garyttierney/me3/commit/3c1a4b02149e8ed17c5f96b950a05ac70a86eebc) [StepSecurity] Apply security best practices
  > Signed-off-by: StepSecurity Bot <bot@stepsecurity.io

- [5552c1f](https://github.com/garyttierney/me3/commit/5552c1f3d0e4a6fd156d574d48f074120df71758) Require Steam to be running before launching the game in [#136](https://github.com/garyttierney/me3/pull/136)
  > Add `require_steam` fn to the launcher which loads `steam_api64.dll`
  > from the game folder and calls `SteamAPI_Init` to determine if Steam is
  > running and the Steam account has a valid game license.


- [b71e9fd](https://github.com/garyttierney/me3/commit/b71e9fde9a564bb070bca076209aae8f8db18524) Replace export polling with scanning for another injection point



- [9dc14b5](https://github.com/garyttierney/me3/commit/9dc14b5bfa379ed1fedbc618c86f384e41e422fa) Merge remote-tracking branch 'origin/pr-noise' into docs-release-notes-upgrade



### 📚 Documentation

- [1317689](https://github.com/garyttierney/me3/commit/13176892eacacbecc46b32174060f7735a529f84) Update README



- [b4047a0](https://github.com/garyttierney/me3/commit/b4047a087941ba68f757e05b34e691142e4b3b58) Add update instructions to release notes in [#114](https://github.com/garyttierney/me3/pull/114)



- [22c909a](https://github.com/garyttierney/me3/commit/22c909a81c6dde6706d807a64ddda0e59d7ab22d) Surround PGP signature in codeblocks



### ⚙️ Miscellaneous Tasks

- [6453215](https://github.com/garyttierney/me3/commit/6453215fec87a0d535cb61f674d1f2925e8935eb)  *(ci)* Typo in set-version package name in [#152](https://github.com/garyttierney/me3/pull/152)



- [cab259a](https://github.com/garyttierney/me3/commit/cab259aa07beba798500fe33447479d3d1711590)  *(ci)* Include full checkout for changelog



- [4caf8ce](https://github.com/garyttierney/me3/commit/4caf8cea149ecbc95b1a36cb698a07d7ac33198f)  *(ci)* Openssf scorecard scanning workflow in [#131](https://github.com/garyttierney/me3/pull/131)



- [b0f8a08](https://github.com/garyttierney/me3/commit/b0f8a08e04e53874c24919d8008638b545560338)  *(ci)* Publish pre-releases with version number prefix



<!-- generated by git-cliff -->
