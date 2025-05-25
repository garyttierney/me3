# ModProfile Configuration Reference

## Example configuration

```toml
profileVersion = "v1" # (1)!

[[packages]] # (2)!
id = "my-cool-texture-pack" 
source = "mods/MyCoolTexturePack/" 

[[natives]] # (3)!
path = "mods/MyAwesomeMod.dll"
```

1.  This is the version of me3 this profile was written for. It allows older profiles to continue working correctly after breaking changes are made in the profile format.

2. This refers to the [Package](./configuration-reference.md#dependent) definition and is used to configure asset overrides.

3. Native DLLs can also be loaded by me3 to benefit from a stable initialization procedure.

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
		 - *An optional symbol to be called after this native succesfully loads.*
	 - <b id="definitionsnativepropertiesload-after">load_after</b>
		 - Type: `array`
			 - ***Items***
			 - &#36;ref: [#/definitions/Dependent](#/definitions/Dependent)
	 - <b id="definitionsnativepropertiesload-before">load_before</b>
		 - Type: `array`
			 - ***Items***
			 - &#36;ref: [#/definitions/Dependent](#/definitions/Dependent)
	 - <b id="definitionsnativepropertiesoptional">optional</b>
		 - *If this native fails to load and this vakye is false, treat it as a critical error.*
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
			 - &#36;ref: [#/definitions/Dependent](#/definitions/Dependent)
	 - <b id="definitionspackagepropertiesload-before">load_before</b>
		 - *A list of packages that this package should load before.*
		 - Type: `array`
			 - ***Items***
			 - &#36;ref: [#/definitions/Dependent](#/definitions/Dependent)
	 - <b id="definitionspackagepropertiessource">source</b> `required`
		 - *A path to the source of this package.*

### Supports

 - Type: `object`
 - ***Properties***
	 - <b id="definitionssupportspropertiesgame">game</b> `required`
		 - &#36;ref: [#/definitions/Game](#/definitions/Game)
	 - <b id="definitionssupportspropertiessince">since</b> `required`
		 - Type: `string`
