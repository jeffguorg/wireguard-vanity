# 1.3 整体路线图

这节给出全书的地图：我们要写哪些文件、按什么顺序写、每一步怎么验证。

## 工程策略：先 CPU，后 GPU

GPU 代码难调试（没有 `println!`，崩溃不报错）。所以本项目分两步走：

1. **CPU 参考实现**（第 4 章）：纯 Rust 实现 X25519 全套数学，用 RFC 7748 官方测试向量和 `curve25519-dalek` 库对拍验证。这一步把「公钥怎么算」彻底钉死。
2. **GPU 移植**（第 6、7 章）：把验证过的 Rust 代码逐行翻译成 GLSL compute shader，再写一个 Rust 宿主程序喂参数、收结果。最后用 GPU 自检模式把两条实现再次对拍。

CPU 版与 GPU 版共享同一套设计（radix 2^25.5 的 10×u64 limb 表示），所以翻译几乎是机械的——这正是第 3 章背景知识要铺垫的。

## 最终项目结构

```text
vulkan/
├── Cargo.toml      # 依赖：vulkano 0.35 / base64 / curve25519-dalek
├── build.rs        # 构建期用 glslangValidator 把 shader 编译成 SPIR-V
├── shader.comp     # GLSL compute shader（GPU 上跑的搜索核心）
└── src/
    ├── main.rs     # 宿主程序：初始化 Vulkan、喂参数、收结果
    ├── fe.rs       # GF(2^255-19) 域运算（CPU 参考实现）
    ├── x25519.rs   # Montgomery 梯子（CPU 参考实现）
    ├── keygen.rs   # splitmix64 私钥派生（与 shader 逐位一致）
    └── prefix.rs   # base64 前缀 → 字节模式 + 掩码
```

## 每章干什么、怎么验证

| 章 | 内容 | 验证方式 |
|---|---|---|
| 2 | 装好 Rust、Vulkan 驱动、glslang | `vulkaninfo --summary` |
| 4 | CPU 参考实现 | `cargo test`（RFC 7748 + dalek 对拍） |
| 6 | 搜索 shader | `glslangValidator` 编译通过 |
| 7 | 宿主程序 | `--selftest` GPU 与 dalek 对拍 |
| 8 | 实测与调优 | 跑出真实私钥，`wg pubkey` 验证 |

## 跟书姿势

- 第 4、6、7 章是「跟着手写」的核心章节，代码块都是最终文件的逐字内容，按节拼接即为完整文件（个别函数在文件中的位置与讲解顺序不同，书中会注明）。
- 其余章节以解释为主，可以只读不敲。
- 所有命令都在那台 GPU 主机的 ssh 会话里执行。
