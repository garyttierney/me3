---
comments: false
hide:
  - navigation
---

# ModProfile (.me3文件) 配置参考

## 什么是ModProfile (.me3文件)配置？

**ModProfile 配置** 是一个版本化的 TOML 文件，它告诉me3要加载哪些mod，如何加载，以及它支持的游戏。它是你的mod设置的一个清单, 列出了游戏资产覆盖包（packages）和本地的DLL（natives），带有可选的加载顺序。

- **如何使用：** me3 读取ModProfile (.me3文件) 以了解需要加载的模组以及顺序。 您可以通过双击它(在`Windows`中) 或使用 CLI (`me3 launching --profile my-profile.me3`)来启动一个配置文件。
- **版本化：** `profileVersion`字段确保配置文件的格式在发生破坏性变更后，旧版本配置仍能保持兼容性。
- **灵活性：** 配置文件（.me3文件）支持任意位置存储，可引用相对路径或绝对路径，并保持对 me3 新功能特性的兼容。

## 配置示例（.me3文件）

```toml
profileVersion = "v1"

[[packages]]
id = "my-col-texture-pack"
path = 'mods/MyCoolTexturePack/'

[[packages]]
id = "my-col-model-pack"
path = 'mods/MyCoolModelPack/'

[[natives]]
path = 'mods/MyAwesomeMod.dll'
```

## 解析配置示例

- **ProfileVersion**：这是为me3编写此配置文件的版本。 它允许在对配置文件格式进行破坏性更改后，旧版本配置文件继续正常工作。
- **[[packages]]**：每个块定义一个游戏资产覆盖包。 `id` 是包的唯一名称， `path` 指向包含mod文件的文件夹。 您可以通过添加更多的 `[[packages]]` 来添加多个包，每个 `[[packages]]` 都有唯一的`id`。 请注意，我们在这里使用单引号，以避免在Windows路径中转义反斜杠。
- **[[natives]]**：每个块定义一个要加载的 DLL mod。 `path`指向DLL文件。 您可以通过添加更多的 `[[natives]]` 来添加多个DLL mod。

## 参考

下方提供Mod Profile（.me3文件）的格式规范。

--8<-- "schemas/mod-profile.zh.md"
