# 故障排除

首次设置模组时遇到问题是一种常见的体验。 本节将指导您诊断和解决使用`me3`时可能遇到的一些更常见的问题。

!!! warning "第一项检查：常见嫌疑"
    在深入研究之前，明智的做法是快速验证一些常见的错误来源。 通常，问题是您的`.me3`文件、`id`或 `packages`或`path`等关键字中的一个简单的**拼写错误**。 另一个常见的陷阱是**不正确的路径**。 请记住，`[[packages]]`和`[[natives]]`的所有路径都是**相对**于`.me3`文件位置的，因此请确保这些路径准确指向您的mod文件。

---

## 资源

- 针对已知错误和常见问题，请参阅[已知问题与常见问题解答](./faq.md#known-issues)。

## 常见问题

### 病毒警告

me3二进制文件现在使用Certum证书进行代码签名，以减少误报。 如果您的杀毒软件标记me3：

- 验证下载内容来自官方[GitHub 发布](https://github.com/garyttierney/me3/releases)
- 将me3 安装程序和me3 安装目录添加到您的病毒排除项

### 游戏未能启动

- 确保Steam正在运行，然后启动me3
- 重复检查您的 .me3 文件中列出的路径
- (Windows) 运行 (++windows+r++) `me3 info` 以检查安装成功
- (Linux) 验证你的配置文件中是否设置 `windows_binaries_dir` ("~/.config/me3\`)

## 仍在出现问题？

在[讨论板]上提交错误报告或寻求帮助。(https://github.com/garyttierney/me3/discussions/)
