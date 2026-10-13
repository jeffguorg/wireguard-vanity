# wireguard-vanity

用 GPU 暴力搜索「公钥带指定前缀」的 WireGuard 密钥对。两条实现路径：

- `opencl/` — Rust（opencl3）+ OpenCL C kernel
- `vulkan/` — Rust（vulkano）+ GLSL compute shader（glslangValidator 编译为 SPIR-V）

两条路径共享同一套 radix-2^25.5（10×u64 limb）GF(2^255-19) 域运算设计，
Rust / OpenCL C / GLSL 三份代码一一对应。

## 用法

```console
$ cargo run --release -- <前缀> [--count N] [--round-log2 N]
$ cargo run --release -- --selftest [N]   # GPU 与 curve25519-dalek 对拍
```

前缀为 base64 字符集（`A-Za-z0-9+/`），长度 1–43。n 字符前缀期望尝试 64^n 次。

## 正确性

- `cargo test`：RFC 7748 测试向量 + 与 curve25519-dalek 随机对拍
- `--selftest`：GPU kernel 与 curve25519-dalek 对拍
- 产出密钥可用 `echo <私钥> | wg pubkey` 交叉验证

## 教程

- `book-opencl/` — OpenCL 路线 mdbook 教程
- `book-vulkan/` — Vulkan 路线 mdbook 教程

构建：`mdbook build book-opencl`（或 `book-vulkan`）。
