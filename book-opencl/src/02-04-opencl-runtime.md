# 2.4 安装 OpenCL 运行时

这节安装让程序能调用 AMD GPU 的 OpenCL 运行时。OpenCL 本身只是一份标准，真正干活的是显卡厂商提供的实现。

## OpenCL 是什么（一句话版）

OpenCL（Open Computing Language）是一套跨厂商的 GPU 通用计算标准：你用类 C 语言写「kernel」，通过一套 C API 把它交给 GPU 成批执行。AMD、NVIDIA、Intel 各自提供实现。本书用 AMD 的 ROCm 实现。

## 安装

Arch Linux 上一条命令：

```console
$ sudo pacman -S rocm-opencl-runtime
```

它会带上两个关键角色：

- **ROCm OpenCL 运行时**：AMD 的 OpenCL 实现，认识你的 GPU；
- **ICD loader（ocl-icd）**：一个「中间商」，程序只跟它对话，它再把请求转给具体厂商的运行时。这样一个系统里装多个厂商的运行时也不冲突。

> 💡 补给：ICD = Installable Client Driver（可安装的客户端驱动）。机制很简单：每个厂商往 `/etc/OpenCL/vendors/` 丢一个 `.icd` 文件，里面写着自己运行时的库路径；loader 启动时扫这个目录，就知道系统里有哪些 OpenCL 实现可用。一句话：**装了运行时还不认卡，先看这个目录里有没有 `.icd` 文件**。

## 其他发行版速查

本书以 Arch 为准，其他发行版只是包名不同：

| 发行版 | 包名 |
|---|---|
| Arch | `rocm-opencl-runtime` |
| Debian / Ubuntu | `rocm-opencl`（或 `amdgpu-install` 里的 opencl 组件） |
| Fedora | `rocm-opencl` |

## 验证装没装好

装完后看一眼 ICD 文件在不在：

```console
$ ls /etc/OpenCL/vendors/
amdocl64.icd
```

有文件就说明运行时注册成功了。但「运行时注册成功」不等于「认到了 GPU」，下一节用 `clinfo` 做最终确认。
