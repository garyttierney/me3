# Mod Profile (.me3文件) 格式规范


### <a id="ModProfileV1"></a>**`v1版本Profile`** *(`object`)*

- **`profileVersion`** *(必填，`string`)*: 固定: `"v1"`。
- **`supports`** *(必填，`array`)*: 设置要启动的游戏。格式参考：*[Supports](#Supports)*。
- **`natives`** *(非必填，`array`)*: 要加载的dll文件路径列表。格式参考：*[Native](#Native)*。
- **`packages`** *(非必填，`array`)*: 游戏资产覆盖包。格式参考：*[Package](#Package)*。
- **`savefile`** *(非必填，`string`)*: 这个可选字段指定游戏将使用的存档文件的文件名，而不是默认的(例如：Elden Ring中的`ER0000.sl2`)。
- **`start_online`** *(非必填，`boolean`)*: 默认情况下，me3会阻止游戏连接到官方多人游戏匹配服务器。 此功能可重新启用。默认值: `false`。
- **`disable_arxan`** *(非必填，`boolean`)*: 尝试解除 Arxan GuardIT 代码保护，以提高模组稳定性。默认值：`null`。
- **`no_mem_patch`** *(非必填，`boolean`)*: 禁用内存补丁。启用(`no_mem_patch = false`)可以为支持的游戏修补内存限制，以提高模组稳定性。默认值：`null`。
- **`heap_size`** *(非必填，`u32`)*: 覆盖支持的游戏应分配的内存大小（以 MB 为单位）（当 `no_mem_patch = true` 时）。最小值：`0`。默认值：`null`。
- **`debug_properties`**: 调试游戏属性覆盖。参考 *[DebugProperties](#DebugProperties)*. 默认值： `{}`.


### <a id="ModProfileV1Example"></a>**`v1版本配置示例`**
```toml
profileVersion = "v1"

savefile = "MyModdedSave.sl2"
start_online = true

[[supports]]
game = "eldenring"

[[natives]]
path = 'SeamlessCoop/ersc.dll'

[[natives]]
path = 'C:/Users/admin/Desktop/ErdTools.dll'
enabled = false

[[packages]]
id = "默认游戏资产覆盖包"
path = 'eldenring-mods'
enabled = false

[[packages]]
id = "default-eldenring"
path = 'C:/Users/admin/Desktop/mod'
```
!!! warning "注意事项"
    配置文件内所有标点符号都为英文，文件路径(比如`path`)需要用 **单引号** 包裹

### <a id="Supports"></a>**`Supports`** *(`object`)*

- **`game`** *(必填)*: 要启动的游戏。格式参考：*[Game](#Game)*。
- **`since_version`** *(非必填)*: (暂无实际作用)


### <a id="Game"></a>**`Game`** *(`string`)*
me3支持的游戏列表

- **其中一个**
    - 黑暗之魂3 (Steam App ID: 374320). 必须其中一个: `["darksouls3", "ds3"]`.
    - 只狼：(Steam App ID: 814380). 必须其中一个: `["sekiro", "sdt"]`。
    - 艾尔登法环: (Steam App ID: 1245620). 必须其中一个: `["eldenring", "er", "elden-ring"]`。
    - 机甲核心6: (Steam App ID: 1888160). 必须其中一个: `["armoredcore6", "ac6"]`。
    - 黑夜君临: (Steam App ID: 2622380). 必须其中一个: `["nightreign", "nr", "nightrein"]`。

### <a id="Native"></a>**`Native`** *(`object`)*

- **`path`** *(必填，`string`)*: dll文件路径，支持相对路径(相对于.me3文件)和绝对路径。
- **`optional`** *(非必填，`boolean`)*: 如果此dll加载失败且此值为`false`，则将其视为严重错误。默认值: `false`。
- **`enabled`** *(非必填，`boolean`)*: 是否启用此DLL。默认值: `true`。默认启用。
- **`load_before`** *(非必填，`array<Dependent<string>>`)*: 默认值: `[]`，参考[Dependent](#Dependent)。
- **`load_after`** *(非必填，`array<Dependent<string>>`)*: 默认值: `[]`，参考[Dependent](#Dependent)。
- **`initializer`** *(非必填，`NativeInitializerCondition`)*: 可选符号，在dll成功加载后调用。
  - **任意一个**
    - : 参考 *[Native初始化条件](#NativeInitializerCondition)*.
    - *null*
- **`finalizer`** *(非必填，`string`)*: 可选符号，当该dll成功加入卸载队列时将被调用。
- **`load_early`** *(非必填，`boolean`)*: 是否在游戏初始化之前加载它。默认值: `false`。

### <a id="Dependent"></a>**`Dependent`** *(`object`)*

- **`id`** *(必填，`string`)*
- **`optional`** *(必填，`boolean`)*

### <a id="NativeInitializerCondition"></a>**`NativeInitializerCondition`**

- **其中一个**
  - *object*: 不能包含额外属性.
    - **`delay`** *(object, required)*
      - **`ms`** *(integer, format: uint, required)*: 最小值: `0`.
  - *object*: 不能包含额外属性.
    - **`function`** *(string, required)*

### <a id="Package"></a>**`Package`** *(`object`)*

游戏资产覆盖包(相当于mod引擎2中的mod文件夹)

- **`id`** *(非必填，`string`)*: 覆盖包的唯一ID/名称。
- **`enabled`** *(非必填，`boolean`)*: 是否启用。默认值：`true`。默认启用。
- **`path`** *(必填，`string`)*: 游戏资产覆盖包路径。支持相对路径(相对于.me3文件)和绝对路径。
- **`load_before`** *(非必填，`array<Dependent<string>>`)*: 应在此包加载前加载的包ID列表。 默认值: `[]`，参考[Dependent](#Dependent)。
- **`load_after`** *(非必填，`array<Dependent<string>>`)*: 应在此包加载后加载的包ID列表。 默认值: `[]`，参考[Dependent](#Dependent)。

### <a id="DebugProperties"></a>**`DebugProperties`** *(`object`)*
任意的游戏调试属性覆盖。<br> 除非你清楚这意味着什么，以及你设置的属性有什么作用，否则不要使用此功能！可以包含额外属性。

- **额外属性**: 参考 *[Property](#Property)*.

### <a id="Property"></a>**`Property`**
游戏属性的值。

- **任意一种**
  - *string*
  - *boolean*
  - *number*
  - *object*: 可以包含额外属性。
    - **额外属性**: 参考 *[Property](#Property)*.