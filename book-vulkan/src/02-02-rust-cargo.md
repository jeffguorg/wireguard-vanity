# 2.2 安装 Rust 与 cargo 上手

这节在 GPU 主机上装好 Rust 工具链，并用一个玩具项目把 cargo 的四个基本命令过一遍。

## 安装 rustup

Arch 官方源就有：

```console
$ sudo pacman -S rustup
$ rustup default stable
```

第二条命令下载并设置 stable 工具链。验证：

```console
$ rustc --version
rustc 1.xx.x ...
$ cargo --version
cargo 1.xx.x ...
```

## cargo 四连

建一个玩具项目热身：

```console
$ cargo new hello
$ cd hello
```

- `cargo build`：编译，产物在 `target/debug/`；
- `cargo run`：编译并运行；
- `cargo test`：编译并运行所有 `#[test]` 函数；
- `cargo run --release`：开优化编译并运行（慢编译、快运行，跑 GPU 搜索必须加 `--release`）。

## 项目里有什么

```text
hello/
├── Cargo.toml   # 项目清单：名字、版本、依赖
└── src/
    └── main.rs  # 入口，fn main()
```

`Cargo.toml` 里的 `[dependencies]` 段落声明第三方库（crate）。本书会用到三个：`vulkano`（Vulkan 绑定）、`base64`、`curve25519-dalek`（成熟密码学库，给我们当「裁判」）。加依赖后 `cargo build` 会自动从 crates.io 下载编译。

💡 **补给：编译很慢是正常的。** 第一次 build 要编译所有依赖，vulkano 这种大 crate 链要几分钟；之后增量编译就快了。release 模式比 debug 慢好几倍编译时间，但运行速度天差地别——跑密码学穷举一定要 `--release`。

## 编辑代码的环境

ssh 会话里直接用 vim/nano 改文件即可；习惯图形编辑器的话，也可以用 VS Code 的 Remote-SSH 插件从本机连过去，体验与本地开发一致。两种方式本书都不依赖，命令行输出完全一样。

环境齐了，下一节装 Vulkan 驱动和 shader 编译器。
