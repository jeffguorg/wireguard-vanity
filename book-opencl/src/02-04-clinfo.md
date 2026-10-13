# 2.4 用 clinfo 验证

这节用一个小工具确认 OpenCL 运行时真的认到了你的 GPU。这是后面一切的前提——这步不过，先别往下走。

## 安装并运行 clinfo

```console
$ sudo pacman -S clinfo
$ clinfo
```

`clinfo` 会把系统里所有 OpenCL 平台和设备的信息打印出来，内容很多。我们只关心几处，用 grep 过滤：

```console
$ clinfo | grep -E "Platform Name|Device Name|Device Version"
  Platform Name                                 AMD Accelerated Parallel Processing
  Device Name                                   gfx1100
  Device Version                                OpenCL 2.0 AMD-APP ...
```

## 看什么

- **`Platform Name`**：能列出一个平台（这里是 AMD APP 平台），说明 ICD loader 工作正常；
- **`Device Name` 出现 `gfx1100`**：说明运行时认到了 GPU。`gfx1100` 是 RX 7900 XTX（Navi 31）的计算单元代号——**看到它，这节就算成功**；
- **`Device Version`**：OpenCL 版本，2.0 或更高都可以（我们的 kernel 只用 1.2 的特性，7.2 节会解释为什么）。

> 💡 补给：`gfx1100` 这类代号是 AMD GPU 架构的「内核名」，类似 CPU 的型号。RX 7900 XTX/XT 是 gfx1100，RX 7800/7700 是 gfx1101，RX 6900 系列是 gfx1030。`clinfo` 报出任何 gfx 开头的设备名都算成功。

## 常见问题排查

| 症状 | 可能原因 |
|---|---|
| `Number of platforms: 0` | ICD 没注册，检查 `/etc/OpenCL/vendors/` 是否为空 |
| 有平台但没有设备 | 驱动没加载，`ls /dev/kfd` 看设备节点在不在 |
| ssh 下看不到设备但本地可以 | 用户不在 `render`/`video` 组：`sudo usermod -aG render,video $USER`，重新登录 |

全部通过后，第 2 章的环境篇就完工了：你有了一台能 ssh 上去、装着 Rust、GPU 被 OpenCL 认出来的主机。下一章补背景知识，第 4 章正式开始敲代码。
