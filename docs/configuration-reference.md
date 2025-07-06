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
- **Flexibility:** Profiles can be stored anywhere, reference relative or absolute paths, and are forward compatible with new me3 features

## Example configuration

```toml
profileVersion = "v1"

[[packages]]
id = "my-cool-texture-pack"
path = 'mods/MyCoolTexturePack/'

[[packages]]
id = "my-cool-model-pack"
path = 'mods/MyCoolTexturePack/'

[[natives]]
path = 'mods/MyAwesomeMod.dll'
```

## Dissecting the example configuration

- **profileVersion**: This is the version of me3 this profile was written for. It allows older profiles to continue working correctly after breaking changes are made in the profile format.
- **[[packages]]**: Each block defines a package of asset overrides. The `id` is a unique name for the package, and `path` points to the folder containing the mod files. You can add multiple packages by adding more `[[packages]]` blocks, each with a unique `id`. Note that we use single quotes here, to avoid having to escape backslashes in Windows paths.
- **[[natives]]**: Each block defines a native DLL mod to load. The `path` points to the DLL file. You can add multiple natives by adding more `[[natives]]` blocks.

## Reference

See below for a rendered version of the mod profile schema.

--8<-- "schemas/mod-profile.md"
