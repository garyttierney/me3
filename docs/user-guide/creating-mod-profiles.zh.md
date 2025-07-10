# 创建Mod Profiles（.me3文件）

**Mod Profile（.me3文件）** 告诉me3要加载哪些mod以及如何加载这些mod。 本指南将指导您下载mod，设置本地mod目录，并创建Mod Profile（.me3文件）。

我们将安装下列DLL mod：[Fast Launch](https://www.nexusmods.com/eldenringnightreign/mods/30) 、 [Nightreign Alt Saves](https://www.nexusmods.com/eldenringnightreign/mods/4) 和 [Disable Chromatic Aberration](https://www.nexusmods.com/eldenringnightreign/mods/67)

对于内容替换，我们将使用：[Fun Is Allowed](https://www.nexusmods.com/eldenringnightreign/mods/49) 和 [Geralt of Rivia over Wylder](https://www.nexusmods.com/eldenringnightreign/mods/63)

## 第 1 步：准备您的mod文件夹

- 决定您将在哪里存储您的mod文件。 默认情况下，me3 将它们存储在 `%LOCALAPPDATA%/garyttierney/me3/config/profiles` 或 `$HOME/.config/me3/profiles` 中，但 `.me3` 文件可以位于除网络驱动器之外的任何位置。
- 创建一个名为`mod`的文件夹来存储你下载的mod文件。

## 第 2 步：添加您的mod

- 将资产文件(例如`regulation.bin`, `parts/`文件夹) 放入`mod`文件夹中。
- 将`.dll`文件放入`natives`文件夹中。
- 为了更容易的管理，您可以在 `mod` 中使用子文件夹，并在您的配置文件中使用独立的 `[[packages]]` 条目作为参考。 这就更容易添加/移除/更新单个mod。

!!! tip "了解路径"
    Mod Profile（.me3文件）文件内(`path` 在 `[[packages]]` 和 `[[natives]]` 中）引用的任何路径都与 `.me3`文件自身的位置相关。
    只要您在`.me3`文件中使用正确的路径，您就可以将您的mod文件存储在您选择的任何路径中。



!!! warning "DLL mod兼容性"
    一些DLL mod可能对配置方式有自己的限制或要求。 请务必查阅每个mod 的文档。

## 第 3 步：创建您的Mod Profile（.me3文件）

在你的 `Mods` 文件夹中创建一个具有以下内容的新文件 (例如`myprofile.me3`)：

```toml
profileVersion = "v1"

[[supports]]
game = "nightreign"

[[packages]]
id = "nightmods"
path = 'mod'

[[natives]]
path = 'natives/DisableChromaticAberration.dll'

[[natives]]
path = 'natives/SkipIntroLogos.dll'

[[natives]]
path = 'natives/nightreign_alt_saves.dll'
```

此配置文件声明了一个名为 `nightmods` 的游戏资产替换包（使用`mod`文件夹中的所有文件），并列出在`natives`文件夹中个每个`.dll` mod。 我们还声明我们的Mod Profile（.me3文件）支持nightreign（黑夜君临），因此me3知道在使用双击启动时要配置的游戏。

## 第 4 步：运行Mod Profile（.me3文件）

现在Mod Profile（.me3文件）已经设置好了，是时候运行它了。 Windows上的用户可以双击 `.me3`文件加载他们的mod启动游戏，而Linux上的用户需要使用跨平台的 CLI 运行配置：

```shell
> $ me3 launch --auto-detect -p myprofile.me3
```

## 第 5 步：游玩已经打了mod的游戏

![图片](https://github.com/user-attachments/assets/9da0bf73-695d-4f0b-af83-2c88e6328fd3)
