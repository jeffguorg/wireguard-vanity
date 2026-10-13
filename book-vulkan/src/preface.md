# 前言

这本书带你从零开始，用 **Vulkan compute shader** 写一个 GPU 程序：暴力搜索「公钥带指定前缀」的 WireGuard 密钥对（俗称靓号 / vanity 密钥）。

## 最终成果

跟完全书，你会逐字敲出一个完整可运行的 Rust + GLSL 项目 `wg-vanity-vulkan`：

```console
$ cargo run --release -- hSD
使用设备: AMD Radeon RX 7900 XTX (RADV NAVI31, LLVM ...)
前缀 "hSD"：期望尝试次数 ≈ 2^18.0
私钥: YOu8...（base64）
公钥: hSDw...（base64，正好以 hSD 开头）
```

产出私钥可以用官方工具 `wg pubkey` 交叉验证，是真的能拿去用的 WireGuard 私钥。

## 你会学到什么

- 密码学部分：X25519 公钥怎么算出来——有限域 GF(2^255−19) 运算、Montgomery 梯子、费马小定理求逆，全部手写，不依赖现成库（库只用来对拍验证）。
- GPU 部分：Vulkan 的 instance / device / queue / pipeline / descriptor set / push constant / fence 这一整套心智模型，以及怎么用 vulkano 在 Rust 里把它们拼起来。
- 工程部分：先在 CPU 上写一份带单元测试的参考实现，再逐行翻译进 GLSL，最后用 GPU 自检功能与参考实现对拍。

## 需要什么样的机器

- 一台装着 **AMD GPU** 的 Linux 主机。书中假设你 ssh 上去操作，系统是 **Arch Linux**（包管理用 `pacman`，Vulkan 驱动用 Mesa 自带的 RADV）。
- 其他发行版也能跟，只是装包的命令换成你自己的包管理器；NVIDIA / Intel 显卡理论上也行，但本书只在 RADV 上实测过。
- 你自己的电脑（用来 ssh 和看这本书）任意系统均可。

## 读者前提

会基础 Rust 就够了：敲过 `cargo new`，认识 `fn`、`struct`，大致懂所有权。**不需要**懂 GPU 编程、密码学或 Vulkan——这些书内就地补。遇到新手容易卡住的点（`Result` 与 `?`、`unsafe`、`Arc`、字节序……），正文里会用「💡 补给」小段现场解释。

## 关于配套实现

本仓库还有一条等价的 OpenCL 路线（`opencl/` 目录，配套教程 `book-opencl/`）。两条路线共享同一套域运算设计，读完这本再看另一本会很轻松。
