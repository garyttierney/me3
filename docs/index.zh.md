---
comments: false
hide:
  - navigation
  - toc
---

# 欢迎使用me3

**me<sup>3</sup>** 是一个专为游戏运行时修改而设计的框架，专注于艾尔登法环和FROMSOFTWARE的其他游戏。 它是 [Mod Engine 2](https://github.com/soulsmods/ModEngine2) 的继任者。

[下载 :fontawesome-solid-download:](https://github.com/garyttierney/me3/releases/latest){ .md-button .md-button--primary }

## 安装

=== ":fontawesome-brands-windows: Windows"

    **一键安装程序：**

    从[GitHub Releases](https://github.com/garyttierney/me3/releases/latest)获取最新的me3_installer.exe并按照安装向导进行操作。

    **手动安装：**

    1. 下载[Windows便携版](https://github.com/garyttierney/me3/releases/latest)
    2. 将其解压到您选择的本地目录（例如，排除在OneDrive或类似软件之外的目录）。

=== ":fontawesome-brands-linux: Linux / Steam Deck"

    **一行安装命令：**
    ```bash
    curl --proto '=https' --tlsv1.2 -sSfL https://github.com/garyttierney/me3/releases/latest/download/installer.sh | sh
    ```

    **手动安装：**

    1. 下载[Linux便携版](https://github.com/garyttierney/me3/releases/latest)
    2. 将其解压到本地目录：
       ```bash
       tar -xzf me3-linux-amd64.tar.gz
       cd me3-linux-amd64
       ./bin/me3 --windows-binaries-dir ./bin/win64 info
       ```

=== ":fontawesome-brands-apple: macOS"

    me3 通过 [CrossOver®️](https://www.codeweavers.com/crossover)支持macOS 。按照您的 CrossOver环境中的 Windows 安装步骤。

## 快速启动指南

### 1. 安装

选择您上面的平台并遵循安装步骤。

### 2. 设置Mod Profile（.me3文件）

- [创建Mod Profile（.me3文件）](user-guide/creating-mod-profiles.md) - 学习如何下载和配置mod。
- [配置参考](configuration-reference.md) - 完整的配置选项

### 3. 运行一个Mod Profile（.me3文件）

运行您配置的 `.me3` 配置文件，或从启动菜单(Windows) 或命令行启动默认配置：

```shell
me3 launch --auto-detect -p eldenring-default
```

## 需要帮助？

- **首次使用？** 请从我们的[用户指南](user-guide/installation.md)开始
- **有问题？** 检查我们的 [故障排除](user-guide/troubleshooting.md)
- **发现错误？** [报告](https://github.com/garyttierney/me3/discussions/categories/bug-reports)
- **想要新功能？** [请求](https://github.com/garyttierney/me3/discussions/categories/ideas)
