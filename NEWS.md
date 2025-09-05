# NEWS

## v0.8.0

v0.8.0 comes with support for Dark Souls 3 and the ability to use a different savefile per me3 profile.
As Dark Souls 3 does not come with any anti-cheat software to prevent you from loading into an online session with mods a new `start_online` option is available in profiles and will be disabled by default.

## v0.7.0

The highlights of v0.7.0 are BootBoost, a startup time improvement technique by [TKGP](https://www.nexusmods.com/darksouls3/mods/303) originally for Dark Souls 3, the ability to launch .me3 files on Linux desktop environments, and localization of our documentation to Chinese and Polish.

As usual, v0.7.0 comes with a bunch of important bug fixes:

- Support Windows filesystem overrides (e.g. custom Lua scripts, custom mod data, DLL ini files) with packages
- Ensure packages/natives exist before passing them to the mod host
- Don't halt shutdown of the game when reading logs
- Find location of Windows binaries on Linux automatically

Thanks to the new contributors who upstreamed translations or features to this release:

- [jn64](https://github.com/jn64) - Launching .me3 files on Linux
- [Lexcellent](https://github.com/Lexcellent) - Chinese localization of me3.help
- [kalarp](https://github.com/kalarp) - Polish localization of me3.help

## v0.6.0

v0.6.0 introduces support for Sekiro mod profiles, portable installations and fixes a number of issues:

- Excessive memory usage/poor performance
- Compatibility with Seamless Coop
- Support for macOS via Crossover
- Custom Steam compatibility tools (e.g. Proton GE)

Additionally binaries are now signed by a Code Signing certificate issued by Certum and preemptively submitted to VirusTotal.
This should avoid anti-virus vendors flagging me3 as potentially unwanted software going forward.
