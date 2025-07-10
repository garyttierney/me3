# Mod Profile (.me3文件) 格式规范


## <a id="ModProfileV1"></a>**`v1版本`**

- **`profileVersion`** *(必填)*: 只能是: `"v1"`。
- **`supports`** *(必填)*: 设置要启动的游戏。格式参考：*[Supports](#Supports)*。
- **`natives`** *(非必填)*: 将要加载的dll文件路径列表。格式参考：*[Native](#Native)*。
- **`packages`** *(非必填)*: 游戏资产覆盖包。格式参考：*[Package](#Package)*。

## <a id="ModProfileV1Example"></a>**`v1版本配置示例`**
```toml
profileVersion = "v1"

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
    配置文件内所有标点符号都为英文，文件路径(比如`path`)需要用**单引号**包裹


### <a id="Supports"></a>**`Supports`**

- **`game`** *(必填)*: 要启动的游戏。格式参考：*[Game](#Game)*。
- **`since`** *(非必填)*: (暂无实际作用)

### <a id="Game"></a>**`Game`**
me3支持的游戏列表
- **其中之一**
    - 只狼：(Steam App ID: 814380). 必须其中之一: `["sekiro", "sdt"]`。
    - 艾尔登法环: (Steam App ID: 1245620). 必须其中之一: `["eldenring", "er", "elden-ring"]`。
    - 机甲核心6: (Steam App ID: 1888160). 必须其中之一: `["armoredcore6", "ac6"]`。
    - 黑夜君临: (Steam App ID: 2622380). 必须其中之一: `["nightreign", "nr", "nightrein"]`。

### <a id="Native"></a>**`Native`**

- **`path`** *(必填)*: dll文件路径，支持相对路径(相对于.me3文件)和绝对路径。
- **`enabled`** *(非必填)*: 是否启用此DLL。默认值: `true`。默认启用。
- **`finalizer`** *(非必填)*: 这是一个可选的符号（函数指针），当该dll成功加入卸载队列时将被调用。
- **`initializer`** *(非必填)*: 一个可选符号（函数指针），在dll成功加载后调用。
- **`load_after`** *(非必填)*: 默认值: `[]`。
- **`load_before`** *(非必填)*: 默认值: `[]`。
- **`optional`** *(非必填)*: 如果此dll加载失败且此值为`false`，则将其视为严重错误。默认值: `false`。

### <a id="Package"></a>**`Package`**

游戏资产覆盖包，相当于mod引擎2中的mod文件夹

- **`id`** *(必填)*: 覆盖包的唯一名称。
- **`path`** *(必填)*: 游戏资产覆盖包路径。支持相对路径(相对于.me3文件)和绝对路径。
- **`enabled`** *(非必填)*: 是否启用。默认值：`true`。默认启用。
- **`load_before`** *(非必填)*: 应在此包加载前加载的包ID列表。 默认值: `[]`。
- **`load_after`** *(非必填)*: 应在此包加载后加载的包ID列表。 默认值: `[]`。

