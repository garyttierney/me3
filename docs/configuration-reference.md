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

### Dependent

 - Type: `object`
 - ***Properties***
	 - <b id="definitionsdependent-for-stringpropertiesid">id</b> `required`
		 - Type: `string`
	 - <b id="definitionsdependent-for-stringpropertiesoptional">optional</b> `required`
		 - Type: `boolean`

### Game

 - Type: `string`
 - The value is restricted to the following:
	 1. *"elden-ring"*
	 2. *"sekiro"*
	 3. *"dark-souls-3"*

### ModFile

 - *A filesystem path to the contents of a package. May be relative to the [ModProfile] containing it.*
 - Type: `string`
***Native***

 - Type: `object`
 - ***Properties***
	 - <b id="definitionsnativepropertiesenabled">enabled</b>
		 - *Should this native be loaded?*
		 - Type: `boolean`
		 - Default: *true*
	 - <b id="definitionsnativepropertiesfinalizer">finalizer</b>
		 - *An optional symbol to be called when this native successfully is queued for unload.*
		 - Types: `string`, `null`
	 - <b id="definitionsnativepropertiesinitializer">initializer</b>
		 - *An optional symbol to be called after this native successfully loads.*
	 - <b id="definitionsnativepropertiesload-after">load_after</b>
		 - Type: `array`
			 - ***Items***
			 - &#36;ref: [#/definitions/Dependent](./configuration-reference.md#dependent)
	 - <b id="definitionsnativepropertiesload-before">load_before</b>
		 - Type: `array`
			 - ***Items***
			 - &#36;ref: [#/definitions/Dependent](./configuration-reference.md#dependent)
	 - <b id="definitionsnativepropertiesoptional">optional</b>
		 - *If this native fails to load and this value is false, treat it as a critical error.*
		 - Type: `boolean`
		 - Default: *false*
	 - <b id="definitionsnativepropertiespath">path</b> `required`
		 - *Path to the DLL. Can be relative to the mod profile.*

### Package

 - *A package is a source for files that override files within the existing games DVDBND archives. It points to a local path containing assets matching the hierarchy they would be served under in the DVDBND.*
 - Type: `object`
 - ***Properties***
	 - <b id="definitionspackagepropertiesid">id</b> `required`
		 - *The unique identifier for this package..*
		 - Type: `string`
	 - <b id="definitionspackagepropertiesload-after">load_after</b>
		 - *A list of package IDs that this package should load after.*
		 - Type: `array`
			 - ***Items***
			 - &#36;ref: [#/definitions/Dependent](./configuration-reference.md#dependent)
	 - <b id="definitionspackagepropertiesload-before">load_before</b>
		 - *A list of packages that this package should load before.*
		 - Type: `array`
			 - ***Items***
			 - &#36;ref: [#/definitions/Dependent](./configuration-reference.md#dependent)
	 - <b id="definitionspackagepropertiessource">source</b> `required`
		 - *A path to the source of this package.*

### Supports

 - Type: `object`
 - ***Properties***
	 - <b id="definitionssupportspropertiesgame">game</b> `required`
		 - &#36;ref: [#/definitions/Game](./configuration-reference.md#game)
	 - <b id="definitionssupportspropertiessince">since</b> `required`
		 - Type: `string`
