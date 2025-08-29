---
comments: false
hide:
  - navigation
---
# ModProfile (.me3) configuration reference

## What is a ModProfile configuration?

A **ModProfile configuration** is a versioned TOML file that tells me3 which mods to load, how to load them, and the games they support. It acts as a manifest for your mod setup, listing asset override packages and native DLLs with an optional load order.

- **How it's used:** me3 reads the ModProfile to know which mods to load and in what order. You can launch a profile by double-clicking it (Windows) or using the CLI (`me3 launch --profile my-profile.me3`).
- **Versioning:** The `profileVersion` field ensures older profiles remain compatible after breaking changes.
- **Flexibility:** Profiles can be stored anywhere, reference relative or absolute paths, and are forward compatible with new me3 features.

## Example configuration

```toml
profileVersion = "v1"
savefile = "MyModdedSave.sl2"
start_online = true

[[supports]]
game = "eldenring"

[[packages]]
path = 'mods/MyCoolTexturePack/'

[[packages]]
path = 'mods/MyCoolModelPack/'

[[natives]]
path = 'mods/MyAwesomeMod.dll'
```

## Dissecting the example configuration

- **profileVersion**: This is the version of me3 this profile was written for. It allows older profiles to continue working correctly after breaking changes are made in the profile format.
- **savefile**: This optional field specifies the file name of the savefile the game will use instead of the default one (e.g. `ER0000.sl2` in Elden Ring). It's extremely handy for compartmentalizing modded content to avoid save corruption and multiplayer bans. If a file with that name does not already exist, me3 copies and renames an existing base savefile. The default save directory is unchanged.
- **start_online**: By default, me3 prevents the game from connecting to the official multiplayer matchmaking servers. This functionality can be reenabled for use with private server mods like Waygate and DS3OS (it is *not* needed for Seamless Co-op). 
- **[[supports]]**: Each block lists a game supported by this profile. Profiles that list exactly one game can be launched without specifying which game to launch.
- **[[packages]]**: Each block defines a package of asset overrides. `path` points to the folder containing the mod files. You can add multiple packages by adding more `[[packages]]` blocks. Note that we use single quotes here, to avoid having to escape backslashes in Windows paths.
- **[[natives]]**: Each block defines a native DLL mod to load. The `path` points to the DLL file. You can add multiple natives by adding more `[[natives]]` blocks.

## Reference

See below for a rendered version of the mod profile schema.

--8<-- "schemas/mod-profile.md"
