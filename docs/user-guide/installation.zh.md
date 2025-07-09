# 安装

本指南提供了安装`me3`的分步说明，这是一个用于FROMSOFTWARE游戏的mod加载器。 在本指南结束时，您的系统上将运行`me3`，并能够使用ELDEN RING的Mod Profile（.me3文件）。

## 运行安装程序

=== ":fontawesome-brands-windows: Windows"

    在 Windows 上安装 me3 最简单的方法是使用每个版本随附的安装程序。此方法可确保所有必要文件正确放置并配置在您的系统上。

    <0>1. 下载安装程序</0>

    首先，您需要从官方来源下载安装程序。导航到 [me3 GitHub 发布页面](https://github.com/garyttierney/me3/releases/latest)，该页面列出了所有可用版本。

    选择一个版本后，在其"Assets"部分中查找 `me3_installer.exe` 文件并下载。

    ??? warning "浏览器安全警告 (点击打开)"

        下载可执行文件（`.exe`）时，您的网络浏览器可能会显示警告，提示该文件可能有害。如果您直接从官方 `me3` GitHub 存储库下载，通常可以信任该文件。选择"保留"或"仍要下载"等选项（具体措辞因浏览器而异）。请始终验证下载来源是 `https://github.com/garyttierney/me3/`。

    <0>2. 运行安装程序</0>

    `me3_installer.exe` 文件下载完成后，在您的"下载"文件夹（或您保存的任何位置）中找到它，然后双击以启动安装向导。

    安装向导将指导您完成设置。选择安装位置后，点击"Install"开始复制文件。进度条将显示安装状态，完成后将出现最终屏幕。点击"Finish"以关闭安装程序。

=== ":fontawesome-brands-linux: Linux"

    me3 为 Linux 提供了一个 shell 脚本安装程序，该程序从 GitHub 下载便携式安装包，将文件解压到正确的位置，可以作为传统的单行安装程序运行：

    <h3>1. 运行安装脚本</h3>

    ```bash
    curl --proto '=https' --tlsv1.2 -sSfL https://github.com/garyttierney/me3/releases/latest/download/installer.sh | sh
    ```

    <h3>2. 将 me3 二进制文件添加到 PATH</h3>

    通过检查 `me3 info` 是否成功来确保 `me3` 在您的 PATH 中可用。如果不可用，请更新 `PATH` 环境变量以包含 `$HOME/.local/bin`。

## 验证安装

me3 默认情况下将为ELDEN RING 创建一组空配置文件（.me3文件）。
让me3从命令行或双击Windows中的.me3文件启动一个空配置文件，检查安装是否正常工作：

```shell
> $ me3 launch --auto-detect -p eldenring-default
```

请参阅`me3 launch--help` 以了解`auto-detect` 参数等更多信息。

## 接下来是什么？

请检查[配置参考](../configuration-reference.md) 和 [配置指南](./creating-mod-profiles.md)以了解如何用me3开始使用mod的信息。
