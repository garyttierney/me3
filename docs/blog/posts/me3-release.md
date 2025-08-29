---
title: me3 released
date: 2025-05-25
categories:
  - Release Announcements
authors:
  - gtierney
---

# me3 released

v0.2.0 of me3 is now released with the basic functionality expected of a mod loader.
This blog post explores installation, configuration, use of the new release.

!!! tip
    Looking for the installer? See the [user guide](../../user-guide/installation.md).
<!-- more -->

## Introduction

me3 is a new iteration on the [ModEngine](https://github.com/soulsmods/ModEngine2) projects that came before it.
It supports all the features you'd expect from a basic mod loader (loading file overrides, DLL extensions, providing crash dumps and logs) but is built on top of a few new design principles:

- Better user-experience for mod users
- Stable integration for mod authors
- Easy maintenance for me3 developers

### Better UX

The approach to configuring mods in the previous versions of ModEngine was barely documented, error prone, and left a lot of confusion about what files should be placed in what folder.
me3 aims to eliminate a lot of the errors that were possible with the previous versions and hopefully makes the experience of using mods a lot less frustrating.

#### Simpler launch experience

me3 comes with both a Windows Shell integration and a cross-platform command-line interface.
Rather than use scripts that invoke `modengine_launcher` users can double-click a `.me3` file to launch it as long as the profile lists the game that it supports.

For users who aren't on Windows (or users who just prefer a CLI) the `me3` command-line interface can be used to launch a profile:

``` shell
> $ me3 launch --profile modded-elden-ring --game er
```

!!! tip
    The `--game` option can be omitted if the profile only supports a single game.

#### Organization of mods

With me3 a Mod Profile can be placed anywhere, and it can reference paths relative to its own configuration file or provide absolute paths to locations elsewhere on the filesystem.
We've standardized the location of Mod Profiles (although they can still be placed anywhere!) to make it easy to find and launch the profiles that are available.

```shell
> $ me3 profile list
eldenring-default.me3
nightreign-default.me3
```

This also means we can create profiles and store them in the new standardized location:

```shell
> $ me3 profile create -g er my-new-profile
> $ me3 profile show my-new-profile
● Mod Profile
    Path: /home/gtierney/.config/me3/profiles/my-new-profile.me3
    Name: my-new-profile
● Supports
    ELDEN RING: Supported
● Natives
● Packages
```

### Stable integration

me3 has a stable API for mod authors that want to distribute parts of the integration with their mod and generate configurations at runtime.

#### Versioned Mod Profile schema

Our past approach to mod configuration files never considered schema evolution and this puts us in a position where we're unable to make improvements to the format
at risk of breaking existing users.
Mod Profiles are now versioned and are forwards-compatible with every future release of me3.
Whenever a breaking change occurs in the configuration schema the `profileVersion` will be bumped and profiles from prior versions will continue to work.

#### Launcher integration

`me3-launcher.exe` is now responsible for attaching the mod host DLL to the game and can be used standalone as part of a custom launcher to run pre-validated profiles by configuring some environment variables.

```shell
ME3_GAME_EXE=path/to/game.exe ME3_HOST_DLL=path/to/me3-mod-host.dll ME3_HOST_CONFIG_PATH=path/to/attach/config/file me3-launcher.exe
```

The `ME3_HOST_CONFIG_PATH` environment variable points to a TOML file containing lists of pre-sorted natives and packages in the same format expected by the `.me3` Mod Profile format.

### Easier maintenance

The biggest change from previous iterations of ModEngine from a developer perspective is that it is simple to build, test, and run with a single command.
Developers can use the same tools that end-users rely on to launch the game and the project can be built with an appropriate set of build tools and a single `cargo build` command.

## Installation and Usage

me3 comes with installers for both Windows and Linux, you can find them on the [releases page](https://github.com/garyttierney/me3/releases/latest/).
Once you've ran the installation wizard, check the [user guide](../../user-guide/creating-mod-profiles.md) for information on creating a mod profile.

## What's next?

There are still some upcoming features aimed at lifting some of the limitations of modding FROMSOFTWARE titles.
The next tasks I'd like to look at are:

- Integration tests for me3 developers
- Support mods with conflicting BND overrides, but non-conflicting BND entries
- Solution to hosting, distributing, and finding Mod Profiles

## Closing words

me3 wouldn't be released today if it wasn't for everyone who has contributed code, documentation, ideas, and more to the various ModEngine projects over the years.
To everyone involved, thank you, in no particular order:

- [Jari Vetoniemi](https://github.com/Cloudef) - for their work on making ModEngine2 support Proton
- [William Tremblay](https://github.com/tremwil) - feedback and insight on the me3 hook system
- [Vincent Swarte](https://github.com/vswarte) - core contributor and primary developer behind ELDEN RING support
- [Dasaav](https://github.com/dasaav-dsv) - ModEngine 2 contributions, bugfixes, feedback, and ongoing development work
- [ividyon](https://github.com/ividyon) - ModEngine2 contributions and feedback on UX/documentation
- [katalash](https://github.com/katalash) - original author of ModEngine and the VFS hook approach
- [horkrux](https://github.com/horkrux) - Dark Souls 3 debug UI patches and ModEngine2 contributions
- Gote - feedback on end-user experience and documentation
- Everyone else who helped along the way