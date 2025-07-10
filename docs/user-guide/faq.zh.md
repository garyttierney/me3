# 已知问题和常见问题

本节列出了基于真实用户反馈和GitHub报告的常见问题和常见问题。 有关故障排除步骤，请参阅[故障排除](troubleshooting.md)。

## 常见问题

### Mod Profile（.me3文件）文件夹在哪里？

默认的模组配置文件目录在 Linux 上是 `$HOME/.config/me3/profiles`，在 Windows 上是 `%LOCALAPPDATA%\garyttierney\me3\config\profiles`

### 启动器未启动。 我应该怎么做？

仔细检查您的配置文件、防病毒设置，并参阅[故障排除](troubleshooting.md)。

### 如何安装mod？

查看文档[创建Mod Profiles（.me3文件）](./creating-mod-profiles.md)

### 在哪里可以找到我的Mod Profile（.me3文件）？

me3 的全局配置文件在 Linux 上是 `$HOME/.config/me3/me3.toml`，在 Windows 上是 `%LOCALAPPDATA%\garyttierney\me3\config\me3.toml`。

### 如何在me3中使用自定义游戏路径？

`me3 launch`命令可以用来指向自定义游戏可执行文件。 例如：

```shell
> $ me3 launch --auto-detect --skip-steam-init --exe-path="C:/game-archive/eldenring.exe"
```

## 已知问题

### （Linux）me3死机，无法启动

!!! bug "如果配置文件中没有设置`crash_reporting`，启动程序可能会在启动时死机。"
!!! success "确保你的`me3.toml`配置文件包含`crash_reporting=true`或`crash-reporting=false`"

### （Steam Deck）将游戏安装到SD卡后，游戏无法启动

!!! bug "me3 无法识别安装在 SD 卡中的游戏的兼容层路径（compatprefix）"
!!! success "请将游戏安装目录迁移至主存储器，或在您的 Steam 库中创建指向兼容层目录的符号链接"

### me3被杀毒软件隔离

!!! bug "某些防病毒软件可能会将启动器（launcher）或mod host（me3_mod_host.dll）标记为恶意。"
!!! success "在防病毒软件中为启动器（launcher）/mod host（me3_mod_host.dll）添加白名单。 仅从官方来源下载。"

### 退出菜单后，游戏仍在Steam中运行

!!! bug "游戏或启动器进程可能无法始终实现完全退出"
!!! success "手动结束延迟的游戏进程（例如通过Windows上的任务管理器）。"

---

如需更多帮助，请访问[故障排除](troubleshooting.md)，或加入社区讨论。
