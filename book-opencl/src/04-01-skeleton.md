# 4.1 创建项目骨架

这节创建项目、配好依赖、搭好文件结构。从这一节开始，所有代码都要亲自动手敲——本章结束时你会得到一个通过全部单元测试的 CPU 版 X25519。

## 创建项目

```console
$ cargo new wg-vanity-opencl
$ cd wg-vanity-opencl
```

## Cargo.toml

把 `Cargo.toml` 改成下面这样（与仓库最终版逐字一致）：

```toml
[package]
name = "wg-vanity-opencl"
version = "0.1.0"
edition = "2021"

[dependencies]
opencl3 = "0.12"
base64 = "0.22"
curve25519-dalek = "5"

[profile.release]
opt-level = 3
```

三个依赖逐个说：

- **`opencl3 = "0.12"`**：OpenCL 的 Rust 绑定，第 7 章宿主程序用它调度 GPU；
- **`base64 = "0.22"`**：打印密钥时的 base64 编码；
- **`curve25519-dalek = "5"`**：成熟的参考实现，只用在测试和自检里当「裁判」。

`[profile.release] opt-level = 3`：release 模式开到最高优化等级。

> 💡 补给：`edition = "2021"` 是 Rust 的版本代号（2015/2018/2021/2024），决定语言的一些默认行为，照抄即可。`[profile.release]` 段对 `cargo build --release` 生效。

## 文件结构

本章我们写两个模块。先建好文件：

```console
$ touch src/fe.rs src/x25519.rs
```

把 `src/main.rs` **临时**改成：

```rust
mod fe;
mod x25519;

fn main() {
    println!("骨架就位：第 7 章会替换为完整的宿主程序");
}
```

⚠️ 注意：这个 `main.rs` 是**脚手架**，不是最终代码——最终的宿主程序第 7 章逐字给出，那时会加上 `mod keygen; mod prefix;` 并替换整个文件。本章只需要让 `cargo test` 能编译、能跑 fe/x25519 的测试。

> 💡 补给：`mod fe;` 声明「本项目有个模块在 `src/fe.rs` 里」。Rust 不像 Python 自动扫描目录，每个文件都要用 `mod` 显式挂到模块树上；漏了 `mod` 声明，文件里的代码等于不存在。

## 验证骨架

```console
$ cargo build
   Compiling wg-vanity-opencl v0.1.0 ...
    Finished ...
```

编译通过就进入下一节，开始写 `fe.rs`。
