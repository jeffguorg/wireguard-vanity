# Summary

[前言](preface.md)

# 第 1 章 项目概览

- [1.1 什么是 vanity 公钥](01-01-what-is-vanity.md)
- [1.2 难度估算](01-02-difficulty.md)
- [1.3 整体路线图](01-03-roadmap.md)

# 第 2 章 环境与工具

- [2.1 ssh 登录 GPU 主机](02-01-ssh.md)
- [2.2 安装 Rust 与 cargo 上手](02-02-rust.md)
- [2.3 安装 OpenCL 运行时](02-03-opencl-runtime.md)
- [2.4 用 clinfo 验证](02-04-clinfo.md)

# 第 3 章 背景知识

- [3.1 WireGuard 密钥对与 base64](03-01-wireguard-base64.md)
- [3.2 X25519 是什么](03-02-x25519.md)
- [3.3 大数怎么装进 u64：limb 与 radix 2^25.5](03-03-limbs.md)
- [3.4 进位与模 2^255-19](03-04-carry-mod.md)
- [3.5 Montgomery ladder](03-05-ladder.md)
- [3.6 费马小定理求逆与加法链](03-06-invert.md)
- [3.7 GPU 为什么适合穷举](03-07-why-gpu.md)

# 第 4 章 CPU 参考实现

- [4.1 创建项目骨架](04-01-skeleton.md)
- [4.2 fe.rs：Fe 类型与加减](04-02-fe-add-sub.md)
- [4.3 fe.rs：carry 进位](04-03-fe-carry.md)
- [4.4 fe.rs：mul](04-04-fe-mul.md)
- [4.5 fe.rs：sq](04-05-fe-sq.md)
- [4.6 fe.rs：invert](04-06-fe-invert.md)
- [4.7 fe.rs：to_bytes / from_bytes](04-07-fe-bytes.md)
- [4.8 x25519.rs：clamp 与梯子主体](04-08-x25519.md)
- [4.9 单元测试：RFC 7748 与 dalek 对拍](04-09-tests.md)

# 第 5 章 OpenCL 入门

- [5.1 平台/设备/上下文/命令队列](05-01-model.md)
- [5.2 kernel、work-item、global id](05-02-kernel-workitem.md)
- [5.3 buffer 与 host↔device 传输](05-03-buffer.md)
- [5.4 opencl3 crate 速览](05-04-opencl3.md)

# 第 6 章 搜索 kernel

- [6.1 kernel 总体结构](06-01-structure.md)
- [6.2 splitmix64 与 clamp 的逐位复刻](06-02-keygen.md)
- [6.3 域运算从 Rust 到 OpenCL C 的对照](06-03-fe-port.md)
- [6.4 梯子与求逆的移植](06-04-ladder-port.md)
- [6.5 前缀匹配：为什么不在 GPU 上做 base64](06-05-prefix-match.md)
- [6.6 命中回传：atomic_inc 与 64 个 slot](06-06-hits.md)
- [6.7 selftest_keys 入口](06-07-selftest.md)

# 第 7 章 宿主程序

- [7.1 命令行解析](07-01-args.md)
- [7.2 初始化与编译 kernel](07-02-init.md)
- [7.3 主循环：写参数 → 发射 → 读结果 → 打印速率](07-03-main-loop.md)
- [7.4 selftest：与 dalek 对拍](07-04-selftest.md)
- [7.5 输出与 wg pubkey 交叉验证](07-05-verify.md)

# 第 8 章 运行、性能与练习

- [8.1 实测数据与预估时间表](08-01-benchmark.md)
- [8.2 调优方向与练习题](08-02-tuning.md)
