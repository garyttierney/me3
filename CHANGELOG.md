# Changelog

All notable changes to this project will be documented in this file.
<!-- markdown-link-check-disable -->
<!-- ignore lint rules that are often triggered by content generated from commits / git-cliff -->
<!-- markdownlint-disable line-length no-bare-urls ul-style emphasis-style -->
## me3 - [v0.6.0](https://github.com/garyttierney/me3/releases/v0.6.0) - 2025-06-29

### 🚀 Features

- [83b3749](https://github.com/garyttierney/me3/commit/83b37493a4e678722f87c6ccc9e876c640415390)  *(host)* Use closure ffi in [#208](https://github.com/garyttierney/me3/pull/208)



- [5fb9e31](https://github.com/garyttierney/me3/commit/5fb9e31958cfccf1bcdb31691bd02c1eea8f82bb)  *(sekiro)* Support Sekiro in [#187](https://github.com/garyttierney/me3/pull/187)


  > Add first-class support for Sekiro mods, and `sekiro` (alias `sdt`) as a
  > CLI flag.

  > Remove unnecessary asset override hook (that did not work in Sekiro and
  > did nothing in other games).


- [de5a57c](https://github.com/garyttierney/me3/commit/de5a57cabfdc096bb626efaaa46bd4ae521ddd47)  *(sekiro)* Add Sekiro as a supported game



- [6cb093b](https://github.com/garyttierney/me3/commit/6cb093bbbbab3ed94469c2461b0e1f19909710ae) Authenticode signatures for Windows binaries in [#238](https://github.com/garyttierney/me3/pull/238)


  > Until now the binaries we've been distributing have been unsigned and as
  > a result lack authenticity. This means we're often being flagged by AV
  > vendors and our reputation effectively resets anytime we produce a
  > "different" binary.

  > Now we've been issued a code signing certificate by Certum that will be
  > used to sign release binaries.


- [ae60bbe](https://github.com/garyttierney/me3/commit/ae60bbe6043e9232315b7097a53b8be1d4a0c4a0) Allow skipping SteamAPI_Init() in launcher in [#226](https://github.com/garyttierney/me3/pull/226)



- [65aec10](https://github.com/garyttierney/me3/commit/65aec1091dfd53fecab92f4ef92644f2a4cec4cb) Support for custom steam compatibility tools in [#217](https://github.com/garyttierney/me3/pull/217)



- [fdf35c9](https://github.com/garyttierney/me3/commit/fdf35c92f9c34e86153f6e741c8b4b58e70e6209) Create portable distributions in [#186](https://github.com/garyttierney/me3/pull/186)


  > Adds a packaging process for portable distributions on both Windows and
  > Linux


- [354c1b7](https://github.com/garyttierney/me3/commit/354c1b798b70af28d690686c666850c7045ffca7) Capture windows exceptions during native mod loads in [#180](https://github.com/garyttierney/me3/pull/180)


  > When we load a native mod there's potential for it to raise a
  > Windows exception and crash the mod host. Now we catch that exception
  > and display a warning to the user that the mod may not be working as
  > expected.


### 🐛 Bug Fixes

- [c93a84d](https://github.com/garyttierney/me3/commit/c93a84d242ccfb5b2f939419626a51222bf92b76)  *(cli)* Platform-specific behavior in [#194](https://github.com/garyttierney/me3/pull/194)



- [3f56843](https://github.com/garyttierney/me3/commit/3f568435bde7e24844d9625b9b39189101d449fe)  *(cli)* Correctly handle `--exe` flag on Windows and Linux



- [c85342c](https://github.com/garyttierney/me3/commit/c85342c9a47f60c9856b138a1eb92c7eb09aabeb)  *(cli)* Enable ANSI escape codes in Windows terminals



- [4049489](https://github.com/garyttierney/me3/commit/404948953e0d052f7465df82d38f644c305ebca3)  *(host)* Override assets from ER DLC ebls in [#220](https://github.com/garyttierney/me3/pull/220)


  > Fixes files found exclusively in DLC.bdt (SOTE) not being overriden by me3.


- [cddf133](https://github.com/garyttierney/me3/commit/cddf1334acdfa5e3a3612ee4c1a40d3f588ff440)  *(host)* Use a more suitable memory location for storing thunk data pointers in [#201](https://github.com/garyttierney/me3/pull/201)



- [d08f2e7](https://github.com/garyttierney/me3/commit/d08f2e7374fbf82a1a78c3c544a4344fa604b869)  *(host)* Use NtTib.ArbitraryUserPointer to store thunk data



- [15052cb](https://github.com/garyttierney/me3/commit/15052cbdac59ebce994642fb8f94103d620756f2)  *(linux)* Prevent prompt spam when no tty is available in [#184](https://github.com/garyttierney/me3/pull/184)


  > Make sure we have an interactive terminal before prompting for input.

  > Fixes #183.

- [ffd74b4](https://github.com/garyttierney/me3/commit/ffd74b4570b8fd54a74836ede00b4f74ff946820)  *(windows)* Correct registry key during uninstall in [#200](https://github.com/garyttierney/me3/pull/200)


  > Fixes #188.

- [b148189](https://github.com/garyttierney/me3/commit/b1481892608707fa176c39eb6a0ce81959c0c544) Excessive CPU utilization from console logs in [#251](https://github.com/garyttierney/me3/pull/251)



- [479bfdd](https://github.com/garyttierney/me3/commit/479bfdd75fbaaaf797117ba5884977e9e6d4bab2) Ensure 64-bit overlay is injected for Proton in [#227](https://github.com/garyttierney/me3/pull/227)


  > Fixes #223.

- [f0083b2](https://github.com/garyttierney/me3/commit/f0083b288a3c410b65c068f6c543db9e60a12ea4) Don't treat filesystem scanning errors as fatal in [#224](https://github.com/garyttierney/me3/pull/224)



- [1a0e488](https://github.com/garyttierney/me3/commit/1a0e488c842e4631bfc49cd331bc6800140da1e8) Reduce overhead of asset hook logging in [#218](https://github.com/garyttierney/me3/pull/218)



- [673905e](https://github.com/garyttierney/me3/commit/673905ed7e7b15207f4595a3f1aa3de108f86be1) Copy-paste errors in Linux portable dist in [#215](https://github.com/garyttierney/me3/pull/215)



- [729dbba](https://github.com/garyttierney/me3/commit/729dbba9858fd8b1a830e6aa0b6846ce5b189025) Add profileVersion='v1' to example profiles



- [68880cf](https://github.com/garyttierney/me3/commit/68880cfdadd4fcffc3d660a42deb1d8f773fefde) Include 'launch' verb in example portable launchers



- [7c76bb2](https://github.com/garyttierney/me3/commit/7c76bb21454b27b218d5a8333f6323a42fde40c8) Update Linux installer to use portable distribution in [#214](https://github.com/garyttierney/me3/pull/214)


  > The Linux distribution previously downloaded individual binaries from
  > the GitHub release that are no longer available. This update downloads
  > the tarball instead and extracts the needed components from there.


- [1ed9c97](https://github.com/garyttierney/me3/commit/1ed9c97a79613aac34dd30850ed0917ae96da4fb) Defer native loading until Steam has initialized in [#212](https://github.com/garyttierney/me3/pull/212)



### 📚 Documentation

- [5370cf4](https://github.com/garyttierney/me3/commit/5370cf416fb07977beb3c675aeec3c49f5128239) Update supported games/platforms in README in [#230](https://github.com/garyttierney/me3/pull/230)



- [1e4a31d](https://github.com/garyttierney/me3/commit/1e4a31d361de5ec03c01076625873a624b7d1762) Add downloads and recent changes badge to README


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



- [d7e8917](https://github.com/garyttierney/me3/commit/d7e891747ed197ed829ffa4a613cccec8f08a68f) Distributed telemetry overhaul in [#113](https://github.com/garyttierney/me3/pull/113)


  > Complete overhaul of the me3 telemetry approach. Now we support
  > distributed spans, capturing backtraces, and associating telemetry with
  > a release version.


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

- [3c1a4b0](https://github.com/garyttierney/me3/commit/3c1a4b02149e8ed17c5f96b950a05ac70a86eebc) [StepSecurity] Apply security best practices


  > Signed-off-by: StepSecurity Bot <bot@stepsecurity.io>


- [5552c1f](https://github.com/garyttierney/me3/commit/5552c1f3d0e4a6fd156d574d48f074120df71758) Require Steam to be running before launching the game in [#136](https://github.com/garyttierney/me3/pull/136)


  > Add `require_steam` fn to the launcher which loads `steam_api64.dll`
  > from the game folder and calls `SteamAPI_Init` to determine if Steam is
  > running and the Steam account has a valid game license.


- [b71e9fd](https://github.com/garyttierney/me3/commit/b71e9fde9a564bb070bca076209aae8f8db18524) Replace export polling with scanning for another injection point



- [9dc14b5](https://github.com/garyttierney/me3/commit/9dc14b5bfa379ed1fedbc618c86f384e41e422fa) Merge remote-tracking branch 'origin/pr-noise' into docs-release-notes-upgrade



- [217b45b](https://github.com/garyttierney/me3/commit/217b45b0da9825a3d818bedddc635455dff3d3b3) Warning when loading NR soundbanks and wems in [#112](https://github.com/garyttierney/me3/pull/112)


  > Temporary hotfix to prevent infinite loops with FIXME comment, I will
  > address the actual issue soon.


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

- [1317689](https://github.com/garyttierney/me3/commit/13176892eacacbecc46b32174060f7735a529f84) Update README



- [b4047a0](https://github.com/garyttierney/me3/commit/b4047a087941ba68f757e05b34e691142e4b3b58) Add update instructions to release notes in [#114](https://github.com/garyttierney/me3/pull/114)



- [22c909a](https://github.com/garyttierney/me3/commit/22c909a81c6dde6706d807a64ddda0e59d7ab22d) Surround PGP signature in codeblocks



- [80e15f4](https://github.com/garyttierney/me3/commit/80e15f4da82f79b2f9950910d637d0a4b9135251) Update acknowledgements for icon artwork



- [6d3c6c4](https://github.com/garyttierney/me3/commit/6d3c6c4a73122aa5dd2903e9455d4e2014b33fd2) Add RELEASE_CHECKLIST



### ⚙️ Miscellaneous Tasks

- [6453215](https://github.com/garyttierney/me3/commit/6453215fec87a0d535cb61f674d1f2925e8935eb)  *(ci)* Typo in set-version package name in [#152](https://github.com/garyttierney/me3/pull/152)



- [cab259a](https://github.com/garyttierney/me3/commit/cab259aa07beba798500fe33447479d3d1711590)  *(ci)* Include full checkout for changelog



- [4caf8ce](https://github.com/garyttierney/me3/commit/4caf8cea149ecbc95b1a36cb698a07d7ac33198f)  *(ci)* Openssf scorecard scanning workflow in [#131](https://github.com/garyttierney/me3/pull/131)



- [b0f8a08](https://github.com/garyttierney/me3/commit/b0f8a08e04e53874c24919d8008638b545560338)  *(ci)* Publish pre-releases with version number prefix



- [3177c9b](https://github.com/garyttierney/me3/commit/3177c9b4de67a08224d54dd819c8e630056dce23)  *(ci)* Make sure PDBs are published



- [9b64a8b](https://github.com/garyttierney/me3/commit/9b64a8b9dfc18edfd4d2a0dfebf3a23b86a3075d)  *(ci)* Always upload coverage to codecov


## me3 - [v0.4.0](https://github.com/garyttierney/me3/releases/v0.4.0) - 2025-06-07

### 🚀 Features

- [31e48cd](https://github.com/garyttierney/me3/commit/31e48cdb9a30d5f3b20a64219c79a6db45ec6520) Add icon and diagnostics verb to windows shell in [#85](https://github.com/garyttierney/me3/pull/85)



- [2abdbf1](https://github.com/garyttierney/me3/commit/2abdbf1d95f1aea69ff9b7922b58fa8240eba8e2) Use icon for me3 profiles on windows



- [6366a58](https://github.com/garyttierney/me3/commit/6366a58da10c42357f772ecb47e196f58b739f50) Log rotation for profile logs


  > Log files are now created in a per-profile directory and rotated
  > automatically after 5 log files are created. Additionally, only one
  > profile may be speciied to `me3 launch`.


- [8f145b5](https://github.com/garyttierney/me3/commit/8f145b570f30ddf7685f937bfe00ccf5b0d7253d) Add 'path' as an alias to 'source' for packages



- [f9527f7](https://github.com/garyttierney/me3/commit/f9527f7ac450e86c471365c4d378daced72b7fbd) Support for armored core 6



### 🐛 Bug Fixes

- [3e45e35](https://github.com/garyttierney/me3/commit/3e45e352f1cc5c9d088d09f384df5f84832ffa0e) Allow blank issues on GH



- [592d3be](https://github.com/garyttierney/me3/commit/592d3beb1ef3c9df454f4eea72ed8f531560bef7) Remove tracing setup log line



- [1aeed8b](https://github.com/garyttierney/me3/commit/1aeed8b2efc040b9bf301ffdb65d7f718d287eee) Sentry propagation



- [0bfc838](https://github.com/garyttierney/me3/commit/0bfc8381b79585c166735e94d85562ba575398e7) Don't create log folders with profile extensions



- [2f15fae](https://github.com/garyttierney/me3/commit/2f15faeb37252d556aaaf3742f4938f7db3f775a) Relax formatting of log files



- [8918f3e](https://github.com/garyttierney/me3/commit/8918f3eac84d6d22be5c27365b5427e9d9a4b16a) Console writing for cli



- [49c6374](https://github.com/garyttierney/me3/commit/49c637462e6e767b066b2e654a79eb5ca10dfd6b) Don't rotate files that aren't log files



- [62768ab](https://github.com/garyttierney/me3/commit/62768ab1ec52ca248fd4c2ddfe89bfc50616c829) Properly associate telemetry with sentry releases



- [bc83a08](https://github.com/garyttierney/me3/commit/bc83a0899de66cf85467f34dbec746f9d297d65f) Disable link checking in release notes template



- [179de3a](https://github.com/garyttierney/me3/commit/179de3a1dcb25b30dd4fd02dcbcc8e3124aa8a98) Typo in README



- [378cd00](https://github.com/garyttierney/me3/commit/378cd006fcdf2b5d1dcdaa07de98100d0d37e3e6) Copy-pasted error message



- [0d886bf](https://github.com/garyttierney/me3/commit/0d886bf9ae5f8302b66f16b2fae670b7cd6a2d42) Enum values and `ReadEbl`



### Other

- [9f8d139](https://github.com/garyttierney/me3/commit/9f8d13961bb2140580942a43a569988fbbd29ce9) Prepare v0.4.0 release in [#88](https://github.com/garyttierney/me3/pull/88)



- [fe65453](https://github.com/garyttierney/me3/commit/fe65453a5ee5ff28587039054c01a63103972080) Add icon to README in [#87](https://github.com/garyttierney/me3/pull/87)



- [5a181f7](https://github.com/garyttierney/me3/commit/5a181f7f3b649aa84eb5797bb607324a04557bbe) OTEL exporter and trace linking in [#86](https://github.com/garyttierney/me3/pull/86)



- [62ae57b](https://github.com/garyttierney/me3/commit/62ae57b91c00d17f3efd5c711eae7e1748bf85b9) Support telemetry linking



- [e521fd3](https://github.com/garyttierney/me3/commit/e521fd3889040fd21478ff3b1493fab0fa2aba39) Rotate old logs on launch in [#84](https://github.com/garyttierney/me3/pull/84)


  > This also restricts the `launch` command to a single profile and uses
  > the profile name as the filename.


- [6c2efa3](https://github.com/garyttierney/me3/commit/6c2efa3473ca6f3192b5cb77c32016cf6675b904) ANSI for console, pretty+plain formatting for log files



- [8c9fb28](https://github.com/garyttierney/me3/commit/8c9fb286a6f753eafdd57244a484b186baeb7af4) Improve release engineering in [#77](https://github.com/garyttierney/me3/pull/77)


  > - [x] Installers/release summary in release notes
  > - [x] Linux installer published from CI


- [3a889bb](https://github.com/garyttierney/me3/commit/3a889bbc6e4a75c180bea9b4a0a59b29849183a6) Publish Linux installer from CI



- [90cb126](https://github.com/garyttierney/me3/commit/90cb126bb7340edf94903724950a72935079123b) Support --exe in addition to --steam-id



- [4bdcb34](https://github.com/garyttierney/me3/commit/4bdcb341b4fc440ab31a09b69094aeaaecb12823) Improve release notes and releng scripts



- [8b222c6](https://github.com/garyttierney/me3/commit/8b222c666410752a20842f157f3d0498a9c7e59e) Capture user feedback and analytics on documentation in [#76](https://github.com/garyttierney/me3/pull/76)



- [8e95d83](https://github.com/garyttierney/me3/commit/8e95d83c376fd1c0b88db0caf7d918729cb11ded) Update README and CHANGELOG for NIGHTREIGN in [#75](https://github.com/garyttierney/me3/pull/75)



- [e04b432](https://github.com/garyttierney/me3/commit/e04b4325abf1953b4f51d0458e76bac8868478b9) Add aliases for game names to mod-protocol



- [699814e](https://github.com/garyttierney/me3/commit/699814ebc54f9390634d02eee26410d7d5e11cc2) Re-add NIGHTREIGN support to README



- [70da381](https://github.com/garyttierney/me3/commit/70da38110e2f55c103547a4bf43a87e94e563be3) New game agnostic asset override approach in [#74](https://github.com/garyttierney/me3/pull/74)


  > Foundation for generically overriding assets including wwise audio files
  > across FromSoftware games.

  > Relies on FD4Singleton scanning, RTTI data (if available), program
  > exports (for wwise, if available), PE image scanning.

  > Does not contain game specific code (as of right now) or use RVAs.
  > Future DS3 support needs using `cxx_stl::msvc2012` types.


- [8e77b7b](https://github.com/garyttierney/me3/commit/8e77b7b30891a4a7a4dbcabed2906a7301293baa) Rustfmt



- [5632e00](https://github.com/garyttierney/me3/commit/5632e001db50c09dcf32c9600ba1801181c6a944) Use structured logging from the `tracing` crate



- [2a91333](https://github.com/garyttierney/me3/commit/2a9133342d016d05842a433501bca9a75d1bb8d0) Return function type instead of pointer



- [8c8375c](https://github.com/garyttierney/me3/commit/8c8375ce4bfbd972682e09d6efe6e3a01edd569b) Use `timeBeginPeriod` to increase sleep resolution



- [c24cfc8](https://github.com/garyttierney/me3/commit/c24cfc83837eb28e2a359e92795844ae339e929e) Don't use submilisecond polling duration



- [de562ad](https://github.com/garyttierney/me3/commit/de562ad3681d49b9c78119b832f0483af807e8bc) Refactor `while` loop into `loop`



- [896b1f8](https://github.com/garyttierney/me3/commit/896b1f8a48fa7a5dc784b05b839e839cc9061943) Explicit `encode_wide_with_nul`



- [de1584a](https://github.com/garyttierney/me3/commit/de1584a5015d1b8ef7cc8c56c1cd4eb2c7d94efb) Apply hooks synchronously



- [79aaf63](https://github.com/garyttierney/me3/commit/79aaf633ab7408e94257c6eccb3ec822cc3eaa9d) Update EblUtility for Nightreign



- [5a4cc7f](https://github.com/garyttierney/me3/commit/5a4cc7f1f28d52f57e06d246bb84e1a569c2e17a) Remove debug print



- [105ce74](https://github.com/garyttierney/me3/commit/105ce7466e9315a14a6bcf94e8b25c24f2a9b25f) Update override mapping to allow for disk-to-disk file overrides (like regulation.bin)



- [4b63e09](https://github.com/garyttierney/me3/commit/4b63e09397150dca2fef47f8de0a4855dc4506cb) Apply rustfmt



- [888ed3f](https://github.com/garyttierney/me3/commit/888ed3fd2024b6b5393f9285a6a7c94efdfd0c74) Better `Debug` impl



- [9fc4125](https://github.com/garyttierney/me3/commit/9fc4125be76163766b75c2d47f76b03404752cce) Don't panic when DlDeviceManager isn't found



- [dfe3edf](https://github.com/garyttierney/me3/commit/dfe3edf19f10375fd241a93d7de459c9e2c6ad39) Remove `instrument` attribute



- [c1e150f](https://github.com/garyttierney/me3/commit/c1e150faa4998d41be88d551ec52d1f3a2706b1f) Remove unused modules



- [4dd64eb](https://github.com/garyttierney/me3/commit/4dd64eb512bac88075821bac95720dc2216ba3ad) New game agnostic asset override approach (normal + wwise)



- [e24c9e8](https://github.com/garyttierney/me3/commit/e24c9e81a9eb0696e0e83006dec110b87e61e7f5) Hooked function providers



- [ee3dd24](https://github.com/garyttierney/me3/commit/ee3dd24f6f96f9be2e22bfe0a8b80e8e2b87f162) PE32 section parsing



- [e4e16fa](https://github.com/garyttierney/me3/commit/e4e16fa8934fc2d9e20ed87b4d50e87c719b3584) Apply clippy suggestions



- [f70da89](https://github.com/garyttierney/me3/commit/f70da89e6d87778e92578109264d10ef0177867a) Poll singletons fn



- [3839cd8](https://github.com/garyttierney/me3/commit/3839cd8314e6ac42782156e90030ac76b6c9fc6b) Update asset lookup dependencies



- [fead392](https://github.com/garyttierney/me3/commit/fead39276859c4d74bf8e453833ed6c26d70f9fe) Update crates



- [51413a8](https://github.com/garyttierney/me3/commit/51413a8240e852e628349b954a870f2d7bb720ae) Add DlUtf16HashString



- [5432cfc](https://github.com/garyttierney/me3/commit/5432cfcc1241f04a09189249a27f0abdad6aa6c5) Fast linear RTTI scanner



- [33b8b17](https://github.com/garyttierney/me3/commit/33b8b17501b9a8834f29305244af4c6c8cf55958) Allow for creating game-compatible allocators



- [02cafe3](https://github.com/garyttierney/me3/commit/02cafe36b992d2b19d6e7d981ab94869d27fc94d) Remove remaining cxx files



### 📚 Documentation

- [e6cb47f](https://github.com/garyttierney/me3/commit/e6cb47fc4c75140bbd2522b3ce064a2f5ccde67f) Add AC6 to README



### ⚙️ Miscellaneous Tasks

- [289339b](https://github.com/garyttierney/me3/commit/289339baa1acadc507741cf59fdc01658c2e2ce4)  *(ci)* Checkout sources during publishing in [#79](https://github.com/garyttierney/me3/pull/79)



- [2840638](https://github.com/garyttierney/me3/commit/2840638c3e7fd1c1b998ff60fa73b116ff637a52)  *(ci)* Fix typo in publishing job in [#78](https://github.com/garyttierney/me3/pull/78)



- [c0d0893](https://github.com/garyttierney/me3/commit/c0d0893b41059422dccf7a516d3df59e99c91d02)  *(ci)* Duplicate workflow name



- [19c85b7](https://github.com/garyttierney/me3/commit/19c85b70d7313957c944041cf45152e6f6a74059)  *(ci)* Permission to download artifacts in publisher


## me3 - [v0.3.0](https://github.com/garyttierney/me3/releases/v0.3.0) - 2025-06-02

### 🐛 Bug Fixes

- [2c3e005](https://github.com/garyttierney/me3/commit/2c3e0051c61db2fb185985c58f5f99bac5f3bfba) Release note generation and PR creation in [#62](https://github.com/garyttierney/me3/pull/62)



- [c466eef](https://github.com/garyttierney/me3/commit/c466eef802a317d736b2fcad7e51fa54942a7f9e) Default profile directory resolution in [#64](https://github.com/garyttierney/me3/pull/64)



- [12a5404](https://github.com/garyttierney/me3/commit/12a54047a53e2481deef9f536a592c370106f925) Typo in attestation output



- [64698ca](https://github.com/garyttierney/me3/commit/64698ca5e3c704f42475d559f7407be16354f760) Some typos and add spellchecking to CI in [#63](https://github.com/garyttierney/me3/pull/63)



- [4921487](https://github.com/garyttierney/me3/commit/4921487c917fe58558f2683253b78473ddbf5a99) Dependency submission



### Other

- [e28e243](https://github.com/garyttierney/me3/commit/e28e2437d90d2a2382740f4b507d158a79edad51) Shell script installer for Linux in [#65](https://github.com/garyttierney/me3/pull/65)


  > Pretty basic, and mostly butchered from rustup Places the `me3` binary
  > in ~/.local/bin and and the windows binaries in
  > `~/.local/share/me3/windows-bin`. If a configuration file doesn't exist
  > it'll create one with `windows_binaries_dir` set and prompt the user to
  > enable crash reporting.

  > Will optionally verify the binaries if the GitHub CLI is available.


- [48fa8dc](https://github.com/garyttierney/me3/commit/48fa8dc162fcc9f2229cbfbfa9ac1bbbe302a603) Merge branch 'main' into feat/linux-installer



- [e21297b](https://github.com/garyttierney/me3/commit/e21297bab894f6d07c399429f3b6ae9bb00f9922) Check if profile_dir is unset after parsing all configuration



- [f9148bb](https://github.com/garyttierney/me3/commit/f9148bb5eb01beb1ec574012f135296dfe2e7d2b) Add blog post on v0.2.0 release in [#66](https://github.com/garyttierney/me3/pull/66)



- [9a29989](https://github.com/garyttierney/me3/commit/9a29989302daffe8f296b01b51fb560853d88a41) Cut off blog post synopsis earlier



- [55bf2b0](https://github.com/garyttierney/me3/commit/55bf2b04df685d2d7ae8b75e2520cb47482bad23) Shell script installer for Linux



- [56f5269](https://github.com/garyttierney/me3/commit/56f5269ba1618bccea2e6fac416ef45dd26ddea3) Add permissions to spellcheck job



- [f1e56cf](https://github.com/garyttierney/me3/commit/f1e56cf27830d85ce34530e891e7a7cced1a3f46) Separate CHANGELOG check so it runs on relabeling


## me3 - [v0.2.0](https://github.com/garyttierney/me3/releases/v0.2.0) - 2025-06-01

### 🚀 Features

- [7a7d04b](https://github.com/garyttierney/me3/commit/7a7d04b63389ace2bbcb7b70891b2a20229ca7ba) Add me3 command-line interface in [#48](https://github.com/garyttierney/me3/pull/48)


  > Introduces a new command-line interface for me3 that can be used to
  > manage profiles, check the status of the me3 install, and run the
  > launcher. See help output of `me3 --help` for more information.

  > Commands supported:

  >     me3 launch
  >     me3 info
  >     me3 profile show
  >     me3 profile create
  >     me3 profile list

  > This also runs as a native binary on Linux hosts and will run the
  > correct commandds uner the hood to set Proton up.


### 🐛 Bug Fixes

- [8dff588](https://github.com/garyttierney/me3/commit/8dff5886e1dfc01637943fc589947a0161369533) Version constraint of workspace packages in [#56](https://github.com/garyttierney/me3/pull/56)



- [a995c81](https://github.com/garyttierney/me3/commit/a995c817176b2975fa1551c892a8fe48ba569698) More dead links



- [d9b5bf1](https://github.com/garyttierney/me3/commit/d9b5bf14616c1836604c32d0bab178a419193396) Dead-links in configuration-reference



- [2f1e7a1](https://github.com/garyttierney/me3/commit/2f1e7a16313dd530faeb41e0fb706ace66ba2cea) Dead links



- [1e2f8d6](https://github.com/garyttierney/me3/commit/1e2f8d66964867dd2bfa35b5456151248a4e6e46) Lints



- [40f6f65](https://github.com/garyttierney/me3/commit/40f6f6599e3d1db5181f96772ca01cbfbbacacfb) Me3 profile show command



- [d8efb8c](https://github.com/garyttierney/me3/commit/d8efb8cce4da1f1277b2651c845ee7c745359eb1) Zombie me3-launcher processes



- [3592721](https://github.com/garyttierney/me3/commit/3592721098a6e0840d8170ae94cc3b6bf856d545) Windows installation registry key name



- [d70925a](https://github.com/garyttierney/me3/commit/d70925a006d6edb13350065815fc2b02e121f4d4) Path to me3 installer in publish action



- [527b336](https://github.com/garyttierney/me3/commit/527b336def6cb1dc556bd6eea2afcd89bd368dba) Args to SBOM upload



- [4e40e27](https://github.com/garyttierney/me3/commit/4e40e278d207dc0412c3883a0fd396a716152055) Generation of Rust SBOMs



- [c70c1ca](https://github.com/garyttierney/me3/commit/c70c1ca06231a206afb8cf30bb2f93109788854c) Prerelease asset upload



- [7262414](https://github.com/garyttierney/me3/commit/7262414bdab2120cfdd0257bb7244104cdcc980f) Prerelease creation



- [afbf683](https://github.com/garyttierney/me3/commit/afbf68340efec3a9c8bb4dce6ae6f89621a44f35) Triggers on publishing workflow



- [2c17c3b](https://github.com/garyttierney/me3/commit/2c17c3b59a0ace36f7000eadb620fdfeeb0e8010) Dependabot updates and dependency review in [#49](https://github.com/garyttierney/me3/pull/49)



- [faea8b2](https://github.com/garyttierney/me3/commit/faea8b2821f83e03dee0ceabdb0604db4154c68f) Dependency review skip condition



- [7898125](https://github.com/garyttierney/me3/commit/7898125a3dd4b1b2b9843a379c2e776472ce5067) Path to mod-host SBOM



- [c562023](https://github.com/garyttierney/me3/commit/c5620237e843819f9c51061033c2dca7c2ef4efd) Args to GH release upload



- [8dcb5f8](https://github.com/garyttierney/me3/commit/8dcb5f8a6bde9873dc49f86580c49af39cad9ed7) Paths to Windows artifacts



- [ea3e2a7](https://github.com/garyttierney/me3/commit/ea3e2a767f5dae622b702b486fd0b05ceac45ef0) Makensis invocation for ubuntu runner



- [c62d330](https://github.com/garyttierney/me3/commit/c62d3300b48fc9b9f5f829df058547b1abd7da12) Makensis invocation



- [18e6b86](https://github.com/garyttierney/me3/commit/18e6b867b5fc42efcfbdd6c9d813ccdad7c92c75) Sentry feature flags for sub-crates



- [715b7c6](https://github.com/garyttierney/me3/commit/715b7c6478001c85f96ad02e0c37b6557ebb0d83) MSVC caching action



- [e1af53a](https://github.com/garyttierney/me3/commit/e1af53a7b606de3d68ab21102ec12ea9c89c8c8e) Clang binary names in CI



- [ec1d02a](https://github.com/garyttierney/me3/commit/ec1d02a26b9d62082ae8b39245d154d536f6c41f) Markdownlint errors



- [fdd29de](https://github.com/garyttierney/me3/commit/fdd29de7a1d27a7043362bd7196827c0ab1ded54) Anchors and rustup installer link



- [db60705](https://github.com/garyttierney/me3/commit/db60705cc720b9ffc52cde83feb7d8c60d0c6048) Camel case capitalization



- [45e7068](https://github.com/garyttierney/me3/commit/45e70689814f93eb8abc1ce498dab9c556acf08b) Markdown extensions



- [1ef22f0](https://github.com/garyttierney/me3/commit/1ef22f0da240260cb03f244db2281cfc6f118fdb) Admonitions



- [63c32cf](https://github.com/garyttierney/me3/commit/63c32cfad66b4d65ac9ddd9c980272f46c70033a) Uploading of release artifacts in [#32](https://github.com/garyttierney/me3/pull/32)



- [558955d](https://github.com/garyttierney/me3/commit/558955d8822da9608aaea2c001e58581c98bc79a) Job cancellation of publishing on main



- [f5aac3d](https://github.com/garyttierney/me3/commit/f5aac3deb64430c5c932aceae6f0e7833fe9039f) Pointer arithmetic for asset hook RVAs



- [b4e0dfd](https://github.com/garyttierney/me3/commit/b4e0dfdd4bc1ddbefd33de2e52c313f1d2c53fcf) Recursion in curried trampolines



- [80daf9c](https://github.com/garyttierney/me3/commit/80daf9cb190865518c282aeace85018c32b871a3) Lints, ensure detours are disabled on Drop



- [6d29167](https://github.com/garyttierney/me3/commit/6d291676918d41f67c4ad112443c4243bff72a86) CXXFLAGS for Linux builds



- [625215c](https://github.com/garyttierney/me3/commit/625215c3ae5a295d66d454b13ca88f3ac31e9036) Sorting of natives/packages with no dependencies



- [df6af9f](https://github.com/garyttierney/me3/commit/df6af9f6744b42512c0d625c232752b5116c4a54) Crash handler being dropped early



- [4f0f21d](https://github.com/garyttierney/me3/commit/4f0f21ddafef3ac74ad16149b5eed19fe076374e) Auto-generation of Prepend impls



- [9643489](https://github.com/garyttierney/me3/commit/9643489b1a6318662ae48a499093f8e6bc72681e) Naked attribute in latest nightly



- [5f5759f](https://github.com/garyttierney/me3/commit/5f5759f73fbfd26499871cb612bf569c3366d9e9) Build



- [cbd6213](https://github.com/garyttierney/me3/commit/cbd62134717b8a45909fdeabfdc285b00d983e6d) Vscode launch task



### Other

- [ebe7325](https://github.com/garyttierney/me3/commit/ebe73255cc0eac4dd0d80800eee70cd1a7c1cb68) Allow users to opt out of  telemetry in [#61](https://github.com/garyttierney/me3/pull/61)



- [108614a](https://github.com/garyttierney/me3/commit/108614aff17646c26ea1c356680c5ff37da937c0) Add an extra line to installer explaining what telemetry is captured



- [fd67acd](https://github.com/garyttierney/me3/commit/fd67acdf18222a2186a0c233f0c6fa4d94deb85f) Respect crash_reporting configuration option



- [c71b1be](https://github.com/garyttierney/me3/commit/c71b1be6cfe386fd39644f6ce75bdc76f7a5e119) Documentation fixes in [#58](https://github.com/garyttierney/me3/pull/58)



- [c5c9842](https://github.com/garyttierney/me3/commit/c5c984281c6a0e3588cf0a90fb5e332e53e71ed9) Merge remote-tracking branch 'origin/main' into docs-fixes



- [694b38a](https://github.com/garyttierney/me3/commit/694b38a0c46071710337553216fe9941a65024aa) Support for self-updates on Windows in [#60](https://github.com/garyttierney/me3/pull/60)



- [1d23660](https://github.com/garyttierney/me3/commit/1d236606f053126a8583161f34d68229cfbc708e) Allow CLI to self-update on Windows



- [3cddc02](https://github.com/garyttierney/me3/commit/3cddc02e824bca98f949f9902519798c4d30bf3d) Support loading natives with initializers in [#59](https://github.com/garyttierney/me3/pull/59)



- [16ba119](https://github.com/garyttierney/me3/commit/16ba11983dd6a13fd92857c7c4a3d69699e49c3d) Update lock file



- [a989a09](https://github.com/garyttierney/me3/commit/a989a09d3eff0bfd4c7c40091930339a739b90bf) Mention NIGHTREIGN is not supported right now



- [b45b617](https://github.com/garyttierney/me3/commit/b45b617734f7d8e9bb80671ba21712d52ff833b0) Grammar fixes in me3 installation instructions



- [41f8d8c](https://github.com/garyttierney/me3/commit/41f8d8c1fcc7cd6fbbe937b9bb232b94ba6b53ad) Installer wizard -> installation wizard



- [2150743](https://github.com/garyttierney/me3/commit/215074385e409161507235690697b12fe85a6dd4) Move installer verification into expandable tip



- [df958c2](https://github.com/garyttierney/me3/commit/df958c2ddfaa55a410490f02a88c703f9d78d947) Shorten quickstart section



- [2c08a7b](https://github.com/garyttierney/me3/commit/2c08a7b63da10422c1e49263157b9c9e6ca746c9) Mod profile -> Mod Profile



- [1021259](https://github.com/garyttierney/me3/commit/10212598867c1945685e17d3e5d0c64d9c685eeb) Use latest version for quickstart link



- [d2f920f](https://github.com/garyttierney/me3/commit/d2f920faf459b5b4439645d922a5f00b2fb78228) Add quickstart and fix dead links



- [d1b79a3](https://github.com/garyttierney/me3/commit/d1b79a3d6678f337b71c045851b525c988344a9f) Split up getting started into user guide



- [0eab1c7](https://github.com/garyttierney/me3/commit/0eab1c72a05640fc898b112faf27cad515568490) Show documentation when installer completes in [#54](https://github.com/garyttierney/me3/pull/54)



- [926192d](https://github.com/garyttierney/me3/commit/926192d2fe662fb441eb6909e2df003ffdcfe765) Update .gitignore



- [9b5e207](https://github.com/garyttierney/me3/commit/9b5e20753c6f79f997a6bd6a900aea2335163165) Add a new workflow check for dead lnks



- [981afea](https://github.com/garyttierney/me3/commit/981afea8d4ef7c3929c22c0c5bec4d99bcd3e066) Add nightreign launcher mapping



- [613a1a0](https://github.com/garyttierney/me3/commit/613a1a0bbfe05e7c2dd07d698e6487937e574b5a) Add some more commands to CI e2e-tests



- [9e07f23](https://github.com/garyttierney/me3/commit/9e07f23f8b9730a00d0f77c5549a47b2261c37be) Update PATH and create profiles as part of installer



- [6360fcd](https://github.com/garyttierney/me3/commit/6360fcde1c599bd47cea74a53ce06381388806b4) Don't rely on absolute paths for CI



- [965c0f9](https://github.com/garyttierney/me3/commit/965c0f9bcfd372a0c8c857e251aaaafe060d0fe5) Add complete path to me3 in e2e test



- [464f780](https://github.com/garyttierney/me3/commit/464f7802944ac31669db687f4f62e3999a9f2441) Add self to path on windows, me3-toml -> me3



- [ff9ffe5](https://github.com/garyttierney/me3/commit/ff9ffe5a90fa2f5d870bda33af60bb8cdee752ce) Update PATH with current user permissions



- [ac7ab8f](https://github.com/garyttierney/me3/commit/ac7ab8fbe66bc4ed73b5554319dc248ac5a84700) Use refreshenv to update PATH in CI



- [b7d1046](https://github.com/garyttierney/me3/commit/b7d1046ad1f5061e974e8894993c3c18188dd1b5) Check localappdata for me3 installation in tests



- [7ddc5ad](https://github.com/garyttierney/me3/commit/7ddc5adcd0eb3dc9e24c7befd5e545240218e90a) Don't rely on Steam app launcher_path



- [5fd0133](https://github.com/garyttierney/me3/commit/5fd013325d19e0fcba2771645d2b3883f28563d7) Modify PATH in onInstSuccess



- [10d4a7b](https://github.com/garyttierney/me3/commit/10d4a7b2a2960ec1462aa9b95c813285b0b74adc) Show installation and PATH in e2e tests



- [f82a570](https://github.com/garyttierney/me3/commit/f82a5706f4aed09190788bfbe2deaa25c7681c90) Re-add --overwrite check



- [d5a2a84](https://github.com/garyttierney/me3/commit/d5a2a84bac54ff461ac9b894753d8751c717adef) Create profile folder when it doesn't already exist



- [c330048](https://github.com/garyttierney/me3/commit/c3300483ad4e604b5eb44079102af3a4cc43cf41) Update mod-profile schema



- [8507ec8](https://github.com/garyttierney/me3/commit/8507ec876f164a241709a4a40c72fe8f8868acf6) Add end-to-end tests to publishing



- [242d04c](https://github.com/garyttierney/me3/commit/242d04c62d2c792842dab1002604d18958c336d2) Nightrein -> neightrein



- [d5ef858](https://github.com/garyttierney/me3/commit/d5ef85856239d83d0831231882dbeda02c3b71f7) Create eldenring and nightrein profile, add me3-cli to PATH



- [bc2dddf](https://github.com/garyttierney/me3/commit/bc2dddf8cdcb41659b86ce111b9a04e2ae4f40f1) Allow creating a profile with a supported game



- [0768dc1](https://github.com/garyttierney/me3/commit/0768dc12c9ddbed9389fad0106621cc3aaa2adf9) Don't raise errors when trying to resolve profile name



- [a7601d1](https://github.com/garyttierney/me3/commit/a7601d1a8b13d51f1f872c144ea4d40007e5035b) Add support for auto-detecting game from mod profile



- [2b3bf45](https://github.com/garyttierney/me3/commit/2b3bf45faa1e2dfdab1f6585b88bdf96fb9bf0dc) Use me3-toml to support file associations on Windows



- [a06eae6](https://github.com/garyttierney/me3/commit/a06eae65b5a55908a7deac567332c579b59f08b6) Use consistent path to me3 appdata



- [88eba37](https://github.com/garyttierney/me3/commit/88eba37b6b159888e28a4c006a4d1ce63c75697c) Cleanup prerelease properly in [#53](https://github.com/garyttierney/me3/pull/53)



- [792439f](https://github.com/garyttierney/me3/commit/792439ff4501c377bdb4a9b2b8b05c8877b4bfa8) Publish Linux binaries with musl in [#52](https://github.com/garyttierney/me3/pull/52)



- [bde5f76](https://github.com/garyttierney/me3/commit/bde5f7661953ebf7c874114bbfe852fba220c3f1) Consistent installer naming, fix dependency review



- [48049e4](https://github.com/garyttierney/me3/commit/48049e40fc918d8fd76cabb510b7c60e9d4d26a1) Run dependency check on Ubuntu



- [7560913](https://github.com/garyttierney/me3/commit/756091307ddf961719c30f40e674091e5e94386d) Install musl toolchain for Linux binaries



- [aa1f924](https://github.com/garyttierney/me3/commit/aa1f9247a4cc60a3e3a475bbc4302003dbfb5930) Always run publishing workflow



- [47bf883](https://github.com/garyttierney/me3/commit/47bf883a66716a458170f73ddf3bb810fb3dfa97) Create prereleases from publishing workflow in [#51](https://github.com/garyttierney/me3/pull/51)



- [0f8d51a](https://github.com/garyttierney/me3/commit/0f8d51ada52e61ac540b660c19b5a66ef77c3bc0) Don't create prereleases from publishing jobs not on main



- [388ddb1](https://github.com/garyttierney/me3/commit/388ddb18cda4b1ac1b71e248cee94c4e612e45f1) Trigger publishing on more release events



- [f827dfa](https://github.com/garyttierney/me3/commit/f827dfad448722844bc7dee3fbd615d704dd0988) Replace YAML configuration with JSON in [#50](https://github.com/garyttierney/me3/pull/50)



- [d681490](https://github.com/garyttierney/me3/commit/d681490e6f237484733a2d7dae1ea4756cddf5e5) Use published event for release uploads



- [cba3bf5](https://github.com/garyttierney/me3/commit/cba3bf53f142589653b77f24166b61b99806b8e5) Generate release notes from CHANGELOG



- [76cb6b1](https://github.com/garyttierney/me3/commit/76cb6b1c60b75c18bd8cdd63e789f9b0389908a6) Update mod profile schema



- [b3b443d](https://github.com/garyttierney/me3/commit/b3b443d1195051f79eff4ab490e69a46397351d6) Build mod-protocol with serde derive feature



- [fc64ad4](https://github.com/garyttierney/me3/commit/fc64ad4b54d2357fab135bc8225aacfcca1f2808) Pass GHA token to GH CLI



- [840bab3](https://github.com/garyttierney/me3/commit/840bab35c16f0928cac794f11903096003b5fa65) Run Clippy and rustfmt on Ubuntu



- [93a70bd](https://github.com/garyttierney/me3/commit/93a70bd1d13041277c7e3903ac4211c56fbd5a91) Remove binary-analysis from workspace list



- [a4627d9](https://github.com/garyttierney/me3/commit/a4627d9353313a4c7e394a1be5ea6020ed67487e) Generate separate attestations for each binary



- [51c2fc0](https://github.com/garyttierney/me3/commit/51c2fc04523a81c4117c7db8c8859ccfd3e89472) Dependency check should have write access to contents



- [6ecb8b6](https://github.com/garyttierney/me3/commit/6ecb8b6463800ac5a8e4efe06bb8bcf896047f16) Add dependabot configuration file



- [5f5918f](https://github.com/garyttierney/me3/commit/5f5918fdab199b678586f7055c2fbbe1bd2780ef) Use manual dependency submission



- [7a49449](https://github.com/garyttierney/me3/commit/7a49449eb5d8fe1d20a02ed88fea01b1b9ad9241) Add tests for CLI output formatting



- [b46d36e](https://github.com/garyttierney/me3/commit/b46d36e91b19e134eb66a4c99cb8a66f1df7c393) Generate attestations for all binaries



- [4b2e0af](https://github.com/garyttierney/me3/commit/4b2e0af19570f182222db2b80eb5d13bfc23edd0) Cleanup caches on PR closure



- [848d87f](https://github.com/garyttierney/me3/commit/848d87f62c8b597a220b0ba086b8401e4fc3601f) Linter fixes



- [7d48af0](https://github.com/garyttierney/me3/commit/7d48af0698079416ded8903ef7672eb38a164423) Install NSIS with apt



- [220acbd](https://github.com/garyttierney/me3/commit/220acbd6d435a17bb4cdf690d7cb999671da118e) Create prereleases on every push to main



- [564433d](https://github.com/garyttierney/me3/commit/564433d5c813a4887be61f212f04103b89fde6e9) Add permissions to workflows missing them



- [fcaa318](https://github.com/garyttierney/me3/commit/fcaa318ccb7f332b11a7e8377a723fcdaa5dba5c) Pin GitHub action versions



- [99c36eb](https://github.com/garyttierney/me3/commit/99c36eb21d6c8aec42c6b70e00cdcd4281968c2a) Cache VS SDK



- [c489b9e](https://github.com/garyttierney/me3/commit/c489b9e744328879c60144422202c6793f2f4933) Ensure CL_FLAGS is set in setup-windows-toolchain



- [d0bf54a](https://github.com/garyttierney/me3/commit/d0bf54a4bcf754e02365f60eff726fab89ba2b69) Build Linux + Windows binaries in publishing job



- [a68a9bb](https://github.com/garyttierney/me3/commit/a68a9bbc5d6f52b34656aaf2387bfad9491e4cba) Install LLVM for Ubuntu builds



- [965d791](https://github.com/garyttierney/me3/commit/965d791e4919d0455cf2e15d49661d8283aa9bc0) Use llvm-lib as AR on Ubuntu



- [65ccfea](https://github.com/garyttierney/me3/commit/65ccfeae2e067c6597d7f0f3ea071e9b09aed4ef) Use lld in linker configuration



- [1304963](https://github.com/garyttierney/me3/commit/13049634ebad26753b2ae30fbcdd2cac420a29f9) Move C++ exception flag to CXXFLAGS



- [f45c331](https://github.com/garyttierney/me3/commit/f45c331dee3effda105dd68eee136e0eff9188d8) Re-enable Linux CI



- [68cd550](https://github.com/garyttierney/me3/commit/68cd550a91a7bd0855ec04a96a9704ae591c637b) Update documentation link



- [e2440f5](https://github.com/garyttierney/me3/commit/e2440f5b06acdecbbccdaab8316c111b81722799) Clearer quickstart link



- [71f6686](https://github.com/garyttierney/me3/commit/71f66861f8be3e53faabd1e4d060726e87888eb1) Remove stray line in README



- [97802a1](https://github.com/garyttierney/me3/commit/97802a1d1b567ecf7f2cd4b8ecafd960bba0238b) Update README



- [35d6add](https://github.com/garyttierney/me3/commit/35d6add2864030ac1a392f83a955cbe3348daabb) Nightrein app ID



- [4c687f7](https://github.com/garyttierney/me3/commit/4c687f728bcd7e7aab3bd7ec2241ce4c0f185397) Regenerate Cargo.lock



- [d26364e](https://github.com/garyttierney/me3/commit/d26364ef433c984fca6b58b4070de932496a558c) Add a check to make sure CHANGELOG is updated in [#47](https://github.com/garyttierney/me3/pull/47)



- [08f89ee](https://github.com/garyttierney/me3/commit/08f89ee592bd783b33bc50e2ac8157bd5b6ce12d) Split PR checks and pull_request_target workflows



- [7daf284](https://github.com/garyttierney/me3/commit/7daf284b7128d6451e35f1bc7f6f37e54ba5a9a9) Allow dependency review to work on forked PRs



- [0745ca7](https://github.com/garyttierney/me3/commit/0745ca78288c86bb7816aa59640bda4257e16325) Re-run changelog check when labels change



- [d5e6d58](https://github.com/garyttierney/me3/commit/d5e6d587beb7bdaade7e2bbaaef3dd0066b626e3) Add permissions and checkout full git history



- [e970895](https://github.com/garyttierney/me3/commit/e970895ecea1f3df841520c44b233ec717a7fb5a) Make PR workflow label shorter



- [0ecd9e4](https://github.com/garyttierney/me3/commit/0ecd9e4c4edebbb00c2a3d296b3ff55742934853) Use base SHA instead of refname



- [f5b72f7](https://github.com/garyttierney/me3/commit/f5b72f7e125e838532e4939442bdef7f6d1f3d74) Model DLString with cxx-stl and remove cxx dependency in [#42](https://github.com/garyttierney/me3/pull/42)


  > Use cxx-stl to model utf8, 16, 32, sjis and eucjp strings with encoding
  > validation and DLStdAllocator support for resource overrides.


- [cf01601](https://github.com/garyttierney/me3/commit/cf0160150cc92e3d9b69478de64a91ff097ee0b8) Merge branch 'main' into feat/better-dlstring



- [2f98bf0](https://github.com/garyttierney/me3/commit/2f98bf028f258bbf7608b6e3a5f70f872780a27a) Structured host->launcher logging in [#43](https://github.com/garyttierney/me3/pull/43)



- [bf9ad1d](https://github.com/garyttierney/me3/commit/bf9ad1d9662c1085ae80426bf89c21155a9410bb) Implement Display instead of ToString as per clippy



- [d77b5e7](https://github.com/garyttierney/me3/commit/d77b5e715a945575b3d74d49b649156f8a776ac4) Capitalization and implementation fixes



- [98d05da](https://github.com/garyttierney/me3/commit/98d05dab457f239810685969211bcb4f1e5a303c) Rewrite asset mapping hook



- [8b040fe](https://github.com/garyttierney/me3/commit/8b040fef61db012e5e431f48070db2eeaf75dff0) Rustfmt



- [2083d9b](https://github.com/garyttierney/me3/commit/2083d9bff6d222a5df8dcd7f95c8e338a6a4f9a2) Make encoding constants private and inlined in the module



- [ad37db6](https://github.com/garyttierney/me3/commit/ad37db64cb3213c5affbb65882acea5b218b3e4c) Use DLString instead of cxx



- [3ac6112](https://github.com/garyttierney/me3/commit/3ac6112d88fea6a370825d1b8f98bfaa66d1e0ad) Remove cxx build.rs



- [6f0a5f5](https://github.com/garyttierney/me3/commit/6f0a5f5b9ae76aa0e32ebd7d80b9f1cb79f302ab) Replace cxx with cxx-stl



- [214d281](https://github.com/garyttierney/me3/commit/214d28199cafc893cc27733d7b6f261628954f29) Add DLString and encoding API



- [1eaa88c](https://github.com/garyttierney/me3/commit/1eaa88c5c1aae6a3a41e7a22ac0c870c66f00cc2) Add DLAllocator API



- [5dee34f](https://github.com/garyttierney/me3/commit/5dee34f8a14a8057fb38f4079ad76da278c1e570) Normalize profile paths instead of canonicalizing in [#41](https://github.com/garyttierney/me3/pull/41)



- [5cd8001](https://github.com/garyttierney/me3/commit/5cd8001f4e1e44cfdf86295b2708e71f333ba659) Add tests for with_context(...) hooks in [#40](https://github.com/garyttierney/me3/pull/40)



- [20c7227](https://github.com/garyttierney/me3/commit/20c7227e3bd8675e335b39523d8d6eb6927fe581) Trampoline pointer should be dereferenced in [#39](https://github.com/garyttierney/me3/pull/39)



- [aca735c](https://github.com/garyttierney/me3/commit/aca735cf3c1b21bb5bcbdf5542c2fdec8182b83f) Set up documentation site in [#37](https://github.com/garyttierney/me3/pull/37)



- [f78959d](https://github.com/garyttierney/me3/commit/f78959d764a1159b65adef689b1dc8ed2b22b061) Add schema links to getting-started



- [79000af](https://github.com/garyttierney/me3/commit/79000af8b794c4bdd8a57b021d09599872546be5) Disable comments on homepage



- [2df3607](https://github.com/garyttierney/me3/commit/2df360712960d4d307ca58b46fd2dbb727d32d2a) Include download links in docs



- [bd34f5f](https://github.com/garyttierney/me3/commit/bd34f5f7f25abba9c07b084793660ee30008d399) Enable theme previews



- [ffe560f](https://github.com/garyttierney/me3/commit/ffe560f8fc7d22ed371b59c27c3004b4c2eb124a) Disable RTD search



- [7c738fd](https://github.com/garyttierney/me3/commit/7c738fd632810c25f7203313ad1ff1f955636c69) Disable readthedocs version selector



- [c115a3f](https://github.com/garyttierney/me3/commit/c115a3ffc8a5e10bcd0526dcd0e3e709ea20d292) Correctly set site_url



- [b71da82](https://github.com/garyttierney/me3/commit/b71da8222dd7a50a2de5074b9a86174bd613bd8e) Copy mkdocs theme configuration



- [1e0ea88](https://github.com/garyttierney/me3/commit/1e0ea88a0ab121d72647b9e993b58a55ae865954) Add icons to getting started guide



- [33d8a68](https://github.com/garyttierney/me3/commit/33d8a68e0614643e789a04eaadccc65a6d84ca60) Enable emoji for fontawesome



- [d0766f9](https://github.com/garyttierney/me3/commit/d0766f95aced2a46146c33bc8bcfc7f7ac64a3e2) Copy mkdocs-material theme palette configuration



- [51784d5](https://github.com/garyttierney/me3/commit/51784d53732717c9fcb7b7111f56f68afb9791a0) Enable recommended extensions



- [355f0ad](https://github.com/garyttierney/me3/commit/355f0ade359e421be394cf954d5d12db803d43de) Enable code highlighting



- [07098bb](https://github.com/garyttierney/me3/commit/07098bb8f10f5785955ae7a4fa83676587c74d78) Enable markdown admonitions



- [b927485](https://github.com/garyttierney/me3/commit/b927485111560da2834ba5843069a6a1e9a161cd) Add configuration reference to navigation



- [ae3910e](https://github.com/garyttierney/me3/commit/ae3910ed21b783cca812e871bac2c71ef1153329) Add basic getting started documentation for Windows



- [66c6e3a](https://github.com/garyttierney/me3/commit/66c6e3a2bfe3ea003891f8dcb643941759f1e0ba) Add blog post description and authors



- [a997453](https://github.com/garyttierney/me3/commit/a997453628cb76042be9c4d4b547c1fadd69e953) Make header less contrasting



- [6e9329f](https://github.com/garyttierney/me3/commit/6e9329fa67cfb0f731cc7dc9ecee331ee622268f) Remove integrated TOC



- [cba24be](https://github.com/garyttierney/me3/commit/cba24be65d72b8559a53f8de4a1f84fca9be305d) Enable navigation/toc features



- [21f2b96](https://github.com/garyttierney/me3/commit/21f2b96656d324bd0ccaac342d660a7b9be0662a) Enable search



- [565f3a6](https://github.com/garyttierney/me3/commit/565f3a672ecc6417738a5c5c813d1c0717aaac5e) Disable doc discussion reactions before comments



- [01f7df4](https://github.com/garyttierney/me3/commit/01f7df4636ffedeb43efd446715a4624be38eb69) Enable GitHub plugins in docs



- [dd4ac0a](https://github.com/garyttierney/me3/commit/dd4ac0a5045a3d08298aa4e8299f39a988298b25) Begin setting up documentation



- [781953d](https://github.com/garyttierney/me3/commit/781953dd2b5a9e8b8abf19787a5a090463e82926) Improve supporting release utilities in [#35](https://github.com/garyttierney/me3/pull/35)



- [ca16a32](https://github.com/garyttierney/me3/commit/ca16a323cbbf61ac9514d6a621a8b96903eb62f0) Make it easier to merge release PRs



- [64ef577](https://github.com/garyttierney/me3/commit/64ef577892d6d6121e7c0900c7b7fb427a809ad7) Include instructions to create tag/release in release PR



- [68b1845](https://github.com/garyttierney/me3/commit/68b18455432263522cae0e16c7938c141295fb0d) Refine release process to support rulesets in [#34](https://github.com/garyttierney/me3/pull/34)



- [e87e66f](https://github.com/garyttierney/me3/commit/e87e66fa3b68e5892728d18f170854f0c5ba581b) Pass GH token to GH cli during release upload



- [f144cba](https://github.com/garyttierney/me3/commit/f144cbaf43f7ea892f5e687ca17d0f55861b73c7) Publish installers during release in [#31](https://github.com/garyttierney/me3/pull/31)



- [190012b](https://github.com/garyttierney/me3/commit/190012b1b68a93955f92104b013e27f84be5ae0d) Only upload PDBs to sentry on release



- [5ff23a0](https://github.com/garyttierney/me3/commit/5ff23a03e8fda98e16bd87b4e69ba8425af1b2a8) Cargo fmt



- [93b6d35](https://github.com/garyttierney/me3/commit/93b6d3518d782429444c207cec0e78c77d4a817b) Don't produce compile errors when SENTRY_DSN is missing



- [99820dd](https://github.com/garyttierney/me3/commit/99820ddc52464b9f1092e711ce524996e98777cc) Capture crash reports with Sentry



- [23f0cdd](https://github.com/garyttierney/me3/commit/23f0cddb451bdd1b05261632c17de0682d0a07b4) Replace Clap with env-var based config



- [ffad718](https://github.com/garyttierney/me3/commit/ffad7186a7b7654bc89f26bb506a8499144fdfda) Publish installer to GH release on tag



- [d6e37cd](https://github.com/garyttierney/me3/commit/d6e37cd0eb7752665274c013a2b44124d17316fd) Remove unnused releng script



- [89d4f89](https://github.com/garyttierney/me3/commit/89d4f895ce01ff6b15bfffd1f950445f5fdb67a5) Create a new branch when creating a release



- [6f9e1b5](https://github.com/garyttierney/me3/commit/6f9e1b56d0501b9f338ff649349c530748a91714) Support for releng



- [e6a7cb7](https://github.com/garyttierney/me3/commit/e6a7cb716f2d56a8c333ab45dca915b0628f1a8b) Prune dependency tree



- [2d43c4a](https://github.com/garyttierney/me3/commit/2d43c4a5bcd4a70928f84a6fa09afea1e49e0a3c) Add missing metadata to all crates



- [9b2ed0d](https://github.com/garyttierney/me3/commit/9b2ed0dd8298d378365bf25d0794461501ba7ebe) Add cargo-release configuration



- [3878ed2](https://github.com/garyttierney/me3/commit/3878ed28915eac96df8c7051e43adc2245c7a0ad) Add labels for GHA workflows



- [91ff355](https://github.com/garyttierney/me3/commit/91ff35581d198061eecb55f3bb63f93b095d08a2) Remove --workspace option from llvm-cov report in [#30](https://github.com/garyttierney/me3/pull/30)



- [47033ae](https://github.com/garyttierney/me3/commit/47033ae54371c7cc03d710339e3d8e2f5737c4f8) Install cargo-nextest for CI



- [b17be90](https://github.com/garyttierney/me3/commit/b17be908b728dd9068114dcb707a5d8edd08ebd4) Only run CI on windows, publish codecov/test results



- [d824698](https://github.com/garyttierney/me3/commit/d824698cb705cbcb1608f06c1e18d3216a46651c) Get clang-cl via clang-tools package for Ubuntu



- [0600a2c](https://github.com/garyttierney/me3/commit/0600a2c24784e393e270f8906162ba8ea797d4ac) Include dependency change summary in PR comments



- [aa30e5a](https://github.com/garyttierney/me3/commit/aa30e5ac088e947ba601a760f2ea4e80b4adfb95) Use GH dependency graph for dependency review



- [5f9f653](https://github.com/garyttierney/me3/commit/5f9f6531fc9c89be729169ba4254c6c099ee2093) Don't enforce newline style in rustfmt



- [7683890](https://github.com/garyttierney/me3/commit/76838905deeb6c0415640e439417d116ccdbc2c0) Regenerate configuration schema



- [a158c23](https://github.com/garyttierney/me3/commit/a158c2312e1f43eff7a261c51260abe73de20919) Cancel concurrent CI builds



- [d78fac8](https://github.com/garyttierney/me3/commit/d78fac8f1e01a423a4fa67ec2e22bce2df77a61b) Include better CI checks



- [5001ce8](https://github.com/garyttierney/me3/commit/5001ce8b62b6c8a0befd20ff37cc2a1583a97940) Install LLVM with sudo



- [e5cad13](https://github.com/garyttierney/me3/commit/e5cad1327b14bda0181a5055552c40c52071e720) Include debug symbols with installer



- [460c8f2](https://github.com/garyttierney/me3/commit/460c8f278d2c6d418b943dc7bfbeb6f98473debb) Canonicalize profile base paths before searching packages/natives



- [4483a56](https://github.com/garyttierney/me3/commit/4483a5678c609a9d3edee78beb13ae76e3f2b965) Merge all mod profiles during launch, return meaningful errors on failure to sort



- [7789724](https://github.com/garyttierney/me3/commit/778972429e0cd6f76b824ed9ecc4f09e95c20654) Install LLVM for Linux builds in CI



- [2ba44e5](https://github.com/garyttierney/me3/commit/2ba44e563f20ed43893cbcae30d3aad1041af376) Update config snapshot tests



- [bc219e4](https://github.com/garyttierney/me3/commit/bc219e44cfb24f44187ca433ae98fac1b9bdfb60) Use version of mod host for installer version



- [cc64090](https://github.com/garyttierney/me3/commit/cc6409031a47b2df3d712f8e636802e5467c351a) Upload installer exe



- [8171e36](https://github.com/garyttierney/me3/commit/8171e365d8b74a16c47bdc1bf1b140275c45719f) Don't require administrator privileges for installation



- [25b546c](https://github.com/garyttierney/me3/commit/25b546cde9ce7df54a66b611b3b374227dbcb666) Absolute path to makensis



- [3b97f98](https://github.com/garyttierney/me3/commit/3b97f9871cc800253d4af0991cd4950979944964) Expose supports via ModProfile public API



- [f16496b](https://github.com/garyttierney/me3/commit/f16496bf53266895dde27e774696d66aa0cff190) Allow a mod profile to state the games it supports


  > It's unlikely that a standard asset bundle mod will make use of this,
  > however it's possible that native mod profiles might. Additionally, it
  > gives us a way to figure out the game we want to tell the launcher to
  > use when invoking a profile via ShellExecute.


- [ea47045](https://github.com/garyttierney/me3/commit/ea47045d41d08efbb3533a9ff906716beb5adb20) Build installer in CI



- [34068b3](https://github.com/garyttierney/me3/commit/34068b33bf3460ad49f705dc3092c523cc822377) Generate me3 installers using NSIS



- [e9a7e3f](https://github.com/garyttierney/me3/commit/e9a7e3fb643573cabe2ed4acea875bbf95337256) Use lld-link for linkage on Linux



- [0c564e8](https://github.com/garyttierney/me3/commit/0c564e8bf3c446ad9c9f749f2349b132418d5777) Setup C++ build environment during CI



- [b09afa2](https://github.com/garyttierney/me3/commit/b09afa241e8f641810cd7051b76175c69c185312) Run CI on main branch



- [c188c1e](https://github.com/garyttierney/me3/commit/c188c1e12043f6797a4356cf2b82db9f69d69a24) Merge remote-tracking branch 'origin/main' into feat/vfs-hook



- [99780dc](https://github.com/garyttierney/me3/commit/99780dc13428364b576155914640e98ad96ed716) Accommodate for changes to unstable features in [#29](https://github.com/garyttierney/me3/pull/29)


  > `asm!` is no longer allowed in naked fns, so I've swapped it for
  > `naked_asm!`. they have also pulled `MaybeUninit::uninit_array` which
  > dll-syringe uses, so I've added a temporary patch entry to a repo that
  > has a fix included.


- [1cefac6](https://github.com/garyttierney/me3/commit/1cefac69f6346cc2fb0c53d7561324706a60a219) Allow conversion of any error type to AttachError



- [f355707](https://github.com/garyttierney/me3/commit/f355707c3d4f9cd23f165f73eee7c66aae38131a) Code cleanup



- [812d440](https://github.com/garyttierney/me3/commit/812d44048ac20279552201b576c224508f6d02b6) Merge branch 'feat/crash-handling' into feat/vfs-hook in [#24](https://github.com/garyttierney/me3/pull/24)



- [f40fd7c](https://github.com/garyttierney/me3/commit/f40fd7c2626e0ffbce905f9204f055ce93b2ec92) Crash handling and support for tracing messages from mod-host


  > Allows the mod-host to signal to the launcher that it wants to perform a
  > crash dump, and also send its own log messages to the same log
  > collection mechanism used by the launcher.


- [04e9f82](https://github.com/garyttierney/me3/commit/04e9f8222257b37510e398e162a3cf8882c2468e) Pass trampolines into hook closures via currying in [#22](https://github.com/garyttierney/me3/pull/22)



- [5bdfe77](https://github.com/garyttierney/me3/commit/5bdfe770855edf437fee9b12395a43a97b36b495) Make paths absolute, probably properly



- [399ab53](https://github.com/garyttierney/me3/commit/399ab53c2350d0360548cbc13a2d3e0dc7cc482c) Reimplement the override logic for the wwise hook, swap fixed pointers for RVA and dynamic bases using GetModuleHandleA



- [b8e0624](https://github.com/garyttierney/me3/commit/b8e06240749caace4ea598e899ec104b685a5e82) Added tests for wwise rewrites, setup wwise hook



- [b02c77b](https://github.com/garyttierney/me3/commit/b02c77bd8fd5ddcf0834e2b8118236e42f78a6da) Implement vfs hook



- [df31a88](https://github.com/garyttierney/me3/commit/df31a887e3b2769585155095745b440f006d735f) Remove unused import of asm



- [286e05f](https://github.com/garyttierney/me3/commit/286e05fc2a4f3370514225d6e2a997f6fe75461b) Merge pull request #17 from garyttierney/build-workflow in [#17](https://github.com/garyttierney/me3/pull/17)


  > Add a build workflow


- [ccac057](https://github.com/garyttierney/me3/commit/ccac057fff4614bc5b487ced3ae170cf078f1038) Install Wine for test running



- [2d35119](https://github.com/garyttierney/me3/commit/2d35119729a33eca45a502493d6319f8690d6bbb) Install LLD for Ubuntu builds



- [d52759a](https://github.com/garyttierney/me3/commit/d52759a3ae537bf4dae7ffc908a08b9ec6e9e708) Merge pull request #19 from vswarte/build-workflow in [#19](https://github.com/garyttierney/me3/pull/19)


  > Update time dep to fix build


- [5c90f3e](https://github.com/garyttierney/me3/commit/5c90f3e1002fee1212a677b269eb5382b6a20eb3) Update time dep



- [902d6c6](https://github.com/garyttierney/me3/commit/902d6c6f8f17f0def952b0a313d687dcb643e111) Add a build workflow



- [af4d14b](https://github.com/garyttierney/me3/commit/af4d14b4cf40634ae16bd8308b73dafb46dcbad7) Merge pull request #16 from garyttierney/fix-runner in [#16](https://github.com/garyttierney/me3/pull/16)


  > Remove runner configuration from Windows target


- [6d0315b](https://github.com/garyttierney/me3/commit/6d0315b0dfff342635c7aeec61e031fa858aab12) Remove runner configuration from Windows target



- [a364eb6](https://github.com/garyttierney/me3/commit/a364eb6c8c7c26d1f0ae4fd0edb04b45e34227ff) Merge pull request #15 from vswarte/chore/toml-config-support in [#15](https://github.com/garyttierney/me3/pull/15)


  > Add TOML support for the mod profiles


- [6a0de25](https://github.com/garyttierney/me3/commit/6a0de25620144b69e50629cdd79830d4e1752389) Run cargo fmt



- [50b9d2c](https://github.com/garyttierney/me3/commit/50b9d2ca12260ea7ba77e07699c3beecb8f862b6) Remove unused test_case macro



- [c7902b8](https://github.com/garyttierney/me3/commit/c7902b8c16a1365174d4dbcbdbaa0f8330a3fd23) Add unit tests for TOML and YAML parsing



- [237460a](https://github.com/garyttierney/me3/commit/237460ae1ab5fa078b0b42a982fcc7a33c5e2d3f) Register schema association for TOML



- [9bc743d](https://github.com/garyttierney/me3/commit/9bc743d8e44d0a6f0f5268162f84a610e3ee2182) Merge branch 'main' into chore/toml-config-support



- [6817ced](https://github.com/garyttierney/me3/commit/6817ced40b5c9eea2c857bf45445b66493e05bc6) Support for native initializer conditions



- [ad56f59](https://github.com/garyttierney/me3/commit/ad56f59aa4c137e24b0ed8a9ac7695ad90a1fd79) Clean up Cargo manifests



- [b21c5e2](https://github.com/garyttierney/me3/commit/b21c5e27e11c09371af8cf6b66d78e905e7df5cb) Toplogical sort natives/packages and return result from attach



- [6ee6391](https://github.com/garyttierney/me3/commit/6ee6391ef048473303cd3b8eb5fb07730f5a20e1) Introduce HookInstaller for ModHost API



- [c3666db](https://github.com/garyttierney/me3/commit/c3666db06d45cb3e9bab13451473aac3280d8aa6) Bounds checks on ThunkPool



- [7bb25e5](https://github.com/garyttierney/me3/commit/7bb25e5dd660fba4598a7c0caf1de493b3975f16) ThunkAllocator -> ThunkPool



- [dc4e2ac](https://github.com/garyttierney/me3/commit/dc4e2ac5feb79f67ab79791b833a01bd8e897b42) Update xwin instructions in README



- [b75f493](https://github.com/garyttierney/me3/commit/b75f4937b66cb1c2dfc289ffb04756fdc0c216ec) Add TOML support for the mod profiles



- [52898d4](https://github.com/garyttierney/me3/commit/52898d4b9f7b015084bff0cdfee7bd8ba72d40d5) Generate function pointers to closures by JITing thunks



- [fccf0a9](https://github.com/garyttierney/me3/commit/fccf0a99625099cd74ece12ab844c80eb8707b4a) Add a file-based logger in addition to stdout



- [4e7821f](https://github.com/garyttierney/me3/commit/4e7821f23fa3c68fc7099cd5c76178fc7684daab) Remove issue filter



- [9951447](https://github.com/garyttierney/me3/commit/995144744f3198cfbddf4cebe34a42c41d625369) Add vscode task to launch me3 launcher



- [0895853](https://github.com/garyttierney/me3/commit/089585369ffdc3bbad001d7e439002950f7e1ed3) Send mod profiles with attech requests



- [c2e2356](https://github.com/garyttierney/me3/commit/c2e235601387bb72e4e953ae7af2433f868a1e24) Include vscode settings for schemas



- [bb187f6](https://github.com/garyttierney/me3/commit/bb187f6c4931a2b91189bba14e2460e4c78e72dd) Add README and CONTRIBUTING



- [c4e6ef5](https://github.com/garyttierney/me3/commit/c4e6ef502776db75d89dbfef6c585b658a28caf4) Initial commit


[0.6.0]: https://github.com/garyttierney/me3/compare/v0.5.0..v0.6.0
[0.5.0]: https://github.com/garyttierney/me3/compare/v0.4.0..v0.5.0
[0.4.0]: https://github.com/garyttierney/me3/compare/v0.3.0..v0.4.0
[0.3.0]: https://github.com/garyttierney/me3/compare/v0.2.0..v0.3.0
[0.2.0]: https://github.com/garyttierney/me3/compare/v0.1.0..v0.2.0

<!-- generated by git-cliff -->
