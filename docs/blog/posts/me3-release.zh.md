---
title: me3 已发布
date: 2025-05-25
categories:
  - 发布公告
authors:
  - gtierney
---

# me3 已发布

me3的v0.2.0版本现已发布，具有模组加载器所需的基本功能。
本文探讨了新版本的安装、配置和使用。

!!! tip
    正在寻找安装程序？ 请查看[用户指南](../../user-guide/installation.md)。

<!-- more -->

## 介绍

me3 是 [ModEngine2](https://github.com/soulsmods/ModEngine2)的新迭代。
它支持您对基本mod加载器所期望的所有功能(加载文件覆盖，DLL 扩展，提供崩溃转储和日志) ，但它是基于一些新的设计原则构建的：

- 为mod用户提供更好的用户体验
- 为mod作者提供稳定的集成
- me3开发者易于维护

### 更好的用户体验

在 ModEngine 的旧版本中， mod配置方法几乎无文档说明，极易出错，且文件目录放置规则混乱不堪。
me3旨在消除以前版本中可能出现的许多错误，并希望使用mod的体验不会那么令人沮丧。

#### 更简单的启动体验

ME3 同时提供 Windows 资源管理器集成和跨平台命令行界面（CLI） 双重操作方案。
用户可以双击“.me3”文件来启动它，而不是使用调用`modengine_launcher`的脚本，只要配置文件列出了它支持的游戏。

对于不使用Windows的用户（或只喜欢CLI的用户），可以使用`me3`命令行界面来启动一个配置文件：

```shell
> $ me3 launch --profile modded-elden-ring --game er
```

!!! tip
    如果配置文件仅支持单个游戏，则可以使用`--auto-detect`选项代替 `--game`来自动检测要运行的游戏。

#### mod组织架构

使用me3，Mod Profile（.me3文件）可以放置在任何地方，它可以引用相对于自己的文件路径（相对路径），也可以使用文件系统上其他位置的绝对路径。
我们已经将Mod Profiles（.me3文件）的位置标准化(尽管它们仍然可以放置在任何地方！) 让我们更容易找到并启动可用的配置文件。

```shell
> $ me3 profile list
eldenring-default.me3
nightreign-default.me3
```

这也意味着我们可以创建配置文件并将其存储在新的标准化位置：

```shell
> $ me3 profile create -g er my-new-profile
> $ me3 profile show my-new-profile
● Mod Profile
    Path: /home/gtierney/.config/me3/profiles/my-new-profile.me3
    Name: my-new-profile
● Supports
    ELDEN RING: Supported
● Natives
● Packages
```

### 稳定集成

me3 拥有一个稳定的 API，供那些希望分发其集成组件并在运行时生成配置的mod作者使用。

#### 版本化的Mod Profile（.me3文件）方案

我们过去处理mod配置文件的方法从未考虑过模式演变，这使我们无法对格式进行改进
存在破坏现有用户的风险。
Mod Profiles（.me3文件）现已实现版本控制，并能兼容me3的所有未来版本。
每当配置格式发生破坏性更改时，`profileVersion`的版本号将会提升，而先前版本的配置文件仍可正常使用。

#### 启动器集成

`me3-launcher.exe`现在负责将mod host DLL（me3_mod_host.dll）附加到游戏中，并且可以作为自定义启动器的一部分单独使用，通过配置一些环境变量来运行预先验证的配置文件。

```shell
ME3_GAME_EXE=path/to/game.exe
ME3_HOST_DLL=path/to/me3-mod-host.dll
ME3_HOST_CONFIG_PATH=path/to/attach/config/file me3-launcher.exe
```

`ME3_HOST_CONFIG_PATH`环境变量指向一个TOML文件，该文件包含预排序的`natives`和`packages`，其格式与 `.me3` Mod Profile格式相同。

### 更容易维护

从开发人员的角度来看，与之前的ModEngine迭代相比，最大的变化是它易于构建、测试和使用单个命令运行。
开发人员可以使用与最终用户相同的工具来启动游戏，并且可以使用一组适当的构建工具和一个 `cargo build`命令来构建项目。

## 安装和使用

me3带有Windows和Linux的安装器，您可以在 [发布页面](https://github.com/garyttierney/me3/releases/latest/)上找到它们。
运行安装向导后，请查看[用户指南](../../user-guide/creating-mod-profiles.md)以获取有关创建Mod Profile（.me3文件）的信息。

## 接下来是什么？

目前仍在开发中的部分新功能，旨在突破FROMSOFTWARE旗下游戏模组制作的部分限制。
接下来我计划处理的任务包括：

- 为me3开发者进行集成测试
- 支持对存在 BND文件覆盖冲突但内部条目无冲突的模组进行兼容处理。
- 托管、分发和查找Mod Profiles（.me3文件）的解决方案

## 结束语

如果不是多年来为各种ModEngine项目贡献代码、文档、想法等的每个人，me3今天就不会发布。
感谢所有参与者，无特定顺序：

- [Jari Vetoniemi](https://github.com/Cloudef) - 使 ModEngine2 支持 Proton
- [William Tremblay](https://github.com/tremwil) - 对me3 hook系统的反馈与洞察
- [Vincent Swarte](https://github.com/vswarte) - 对ELDEN RING支持的核心贡献者和主要开发者
- [Dasaav](https://github.com/dasaav-dsv) - ModEngine 2 贡献、错误修复、反馈和正在进行的开发工作
- [ividyon](https://github.com/ividyon) - ModEngine2对用户体验/文档的贡献和反馈
- [katalash](https://github.com/katalash) - ModEngine 及其 VFS Hook 技术方案的原作者
- [horkrux](https://github.com/horkrux) - Dark Souls 3调试UI补丁和ModEngine2贡献
- Gote - 对最终用户体验和文档的反馈
- 感谢一路相助的所有人
