## Properties


### <a id="ModProfile"></a>**`ModProfile`**


  - **One of**
    - *object*: Refer to *[ModProfileV1](#ModProfileV1)*.
      - **`profileVersion`** *(string, required)*: Must be: `"v1"`.
## Definitions


### <a id="Dependent"></a>**`Dependent`** *(object)*


  - **`id`** *(string, required)*
  - **`optional`** *(boolean, required)*

### <a id="Game"></a>**`Game`** *(string)*
 List of games supported by me3.

  - **One of**
    - : DARK SOULS III (Steam App ID: 374320). Must be one of: `["darksouls3", "ds3"]`.
    - : Sekiro: Shadows Die Twice (Steam App ID: 814380). Must be one of: `["sekiro", "sdt"]`.
    - : Elden Ring (Steam App ID: 1245620). Must be one of: `["eldenring", "er", "elden-ring"]`.
    - : Armored Core VI: Fires of Rubicon (Steam App ID: 1888160). Must be one of: `["armoredcore6", "ac6"]`.
    - : Elden Ring Nightreign (Steam App ID: 2622380). Must be one of: `["nightreign", "nr", "nightrein"]`.

### <a id="ModFile"></a>**`ModFile`** *(string)*
 A filesystem path to the contents of a package. May be relative to the [ModProfile] containing
it.


### <a id="ModProfileV1"></a>**`ModProfileV1`** *(object)*

  - **`savefile`** *(['string', 'null'])*: This optional field specifies the file name of the savefile the game will use instead of the default one (e.g. `ER0000.sl2` in Elden Ring).
  - **`start_online`** *(boolean)*: By default, me3 prevents the game from connecting to the official multiplayer matchmaking servers. This functionality can be reenabled. Default: `false`.
  - **`natives`** *(array)*: Native modules (DLLs) that will be loaded. Default: `[]`.
  - **`packages`** *(array)*: A collection of packages containing assets that should be considered for loading
before the DVDBND. Default: `[]`.
  - **`supports`** *(array)*: The games that this profile supports. Default: `[]`.

### <a id="Native"></a>**`Native`** *(object)*


  - **`enabled`** *(boolean)*: Should this native be loaded? Default: `true`.
  - **`load_early`** *(boolean)*: Should this native be loaded before others? Default: `false`.
  - **`finalizer`** *(['string', 'null'])*: An optional symbol to be called when this native successfully is queued for unload.
  - **`initializer`**: An optional symbol to be called after this native successfully loads.
    - **Any of**
      - : Refer to *[NativeInitializerCondition](#NativeInitializerCondition)*.
      - *null*
  - **`load_after`** *(array)*: Default: `[]`.
  - **`load_before`** *(array)*: Default: `[]`.
  - **`optional`** *(boolean)*: If this native fails to load and this value is false, treat it as a critical error. Default: `false`.
  - **`path`**: Path to the DLL. Can be relative to the mod profile. Refer to *[ModFile](#ModFile)*.

### <a id="NativeInitializerCondition"></a>**`NativeInitializerCondition`**


  - **One of**
    - *object*: Cannot contain additional properties.
      - **`delay`** *(object, required)*
        - **`ms`** *(integer, format: uint, required)*: Minimum: `0`.
    - *object*: Cannot contain additional properties.
      - **`function`** *(string, required)*

### <a id="Package"></a>**`Package`** *(object)*
 A package is a source for files that override files within the existing games DVDBND archives.
It points to a local path containing assets matching the hierarchy they would be served under in
the DVDBND.

  - **`enabled`** *(boolean)*: Enable this package? Default: `true`.
  - **`id`** *(['string', 'null'])*: The unique identifier for this package.
  - **`load_after`** *(array)*: A list of package IDs that this package should load after. Default: `[]`.
  - **`load_before`** *(array)*: A list of packages that this package should load before. Default: `[]`.
  - **`path`**: A path to the source of this package. Refer to *[ModFile](#ModFile)*.

### <a id="Supports"></a>**`Supports`** *(object)*


  - **`game`**: Refer to *[Game](#Game)*.
  - **`since`** *(['string', 'null'])*
