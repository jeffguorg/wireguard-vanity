# 2.3 Vulkan 驱动与 glslang

这节装好 GPU 计算需要的三个系统组件：Vulkan 驱动、验证工具、shader 编译器，并确认驱动认识我们的显卡。

## 安装

Arch 下一条命令：

```console
$ sudo pacman -S vulkan-radeon vulkan-tools glslang
```

- `vulkan-radeon`：Mesa 的 AMD Vulkan 驱动，代号 **RADV**。它是用户态驱动，Vulkan 程序最终通过它和显卡对话。
- `vulkan-tools`：提供 `vulkaninfo` 等诊断命令。
- `glslang`：提供 `glslangValidator`，把 GLSL 编译成 SPIR-V（第 5 章细讲）。

## 验证驱动

```console
$ vulkaninfo --summary
==========
VULKANINFO
==========
...
	deviceName        = AMD Radeon RX 7900 XTX (RADV NAVI31, LLVM ...)
	driverName        = radv
	...
```

`--summary` 只打印概览。看到 `deviceName` 里出现你的显卡、`driverName = radv`，说明驱动栈通了。

💡 **补给：无头主机也能跑 Vulkan。** 我们只跑 compute（计算），不要显示器、不要窗口系统、不要桌面环境。ssh 会话里直接跑就行——这正是 Vulkan compute 适合服务器的原因。

## 确认一个关键特性：shaderInt64

我们的 shader 里全是 64 位整数运算（`uint64_t`），这不是所有 GPU 都支持的。查一下：

```console
$ vulkaninfo | grep shaderInt64
	shaderInt64                              = true
```

输出 `true` 即可。RX 7900 XTX / RADV 没问题；如果哪天换设备跑出 `false`，第 8 章的 32-bit limb 改造方向就是为你准备的。

## 验证 glslangValidator

```console
$ glslangValidator --version
Glslang Version: ...
```

能打印版本号就行。第 5 章会拿它手动编一次 shader 给你看，第 7 章的 `build.rs` 会在每次 `cargo build` 时自动调用它。

至此环境备齐：Rust ✓、Vulkan 驱动 ✓、shader 编译器 ✓。下一章补密码学背景，第 4 章正式开始写代码。
