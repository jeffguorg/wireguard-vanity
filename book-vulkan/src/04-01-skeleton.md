# 4.1 创建项目骨架

这章在 CPU 上把 X25519 全套数学用纯 Rust 写出来并测试。这节建项目、写清单，并把第一个小模块 `keygen.rs`（私钥派生）完整敲出来。

## 建项目

```console
$ cargo new wg-vanity-vulkan
$ cd wg-vanity-vulkan
```

## Cargo.toml

把 `Cargo.toml` 改成下面这样（与最终文件一致）：

```toml
[package]
name = "wg-vanity-vulkan"
version = "0.1.0"
edition = "2021"

[dependencies]
vulkano = "0.35"
base64 = "0.22"
curve25519-dalek = "5"

[profile.release]
opt-level = 3
```

`vulkano` 到第 7 章才用，现在写上是为了让清单一次到位；`curve25519-dalek` 和 `base64` 这章的测试就要用。`opt-level = 3` 确保 release 构建满优化。

## main.rs 占位与 mod 机制

第 4 章我们只关心库代码，`src/main.rs` 先放一个占位（第 7 章会整份替换）：

```rust
mod keygen;

fn main() {}
```

💡 **补给：mod。** Rust 里 `mod keygen;` 声明「本 crate 有一个模块，代码在 `src/keygen.rs`」。不写这行，文件放那儿编译器也不看。fe.rs（4.2 节）和 x25519.rs（4.8 节）到位时会各自补上对应的 `mod` 行。

## keygen.rs：私钥派生

每个 GPU 线程用 `seed + gid × φ` 作为 splitmix64 的状态，连取 4 个 u64 拼成 32 字节，再 clamp。φ 是黄金比例常数（奇数），保证不同 gid 的状态互不重复。CPU 版必须与 shader 逐位一致，因为 selftest 要靠它生成参考答案。

创建 `src/keygen.rs`（测试模块推迟到 4.9 节补敲——它要调用还没写的 `x25519.rs`，与 prefix.rs 的测试推迟到 7.7 是同一模式）：

```rust
//! 与 GPU kernel 共享的私钥派生逻辑。
//!
//! 每个 GPU work-item 用 `seed + gid * φ` 作为 splitmix64 的状态，
//! 连取 4 个 u64 得到 32 字节，再做 X25519 clamp。
//! φ 是黄金比例常数（奇数），保证不同 gid 的状态互不重复。

pub const PHI: u64 = 0x9E37_79B9_7F4A_7C15;

pub fn splitmix64(s: &mut u64) -> u64 {
    *s = s.wrapping_add(PHI);
    let mut z = *s;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// 派生第 gid 个候选私钥（已 clamp），与 kernel 内逻辑逐位一致。
pub fn derive_key(seed: u64, gid: u64) -> [u8; 32] {
    let mut s = seed.wrapping_add(gid.wrapping_mul(PHI));
    let mut w = [0u64; 4];
    for x in &mut w {
        *x = splitmix64(&mut s);
    }
    // clamp，等价于字节级的 k[0] &= 248; k[31] &= 127; k[31] |= 64
    w[0] &= !7;
    w[3] = (w[3] & 0x7FFF_FFFF_FFFF_FFFF) | 0x4000_0000_0000_0000;

    let mut key = [0u8; 32];
    for (i, x) in w.iter().enumerate() {
        key[i * 8..i * 8 + 8].copy_from_slice(&x.to_le_bytes());
    }
    key
}
```

💡 **补给：wrapping_add / wrapping_mul。** splitmix64 靠溢出回绕混洗比特，这是故意的。Rust 默认算术在 debug 下溢出会 panic，所以故意回绕必须写 `wrapping_*`，语义和 GLSL 的 `uint64_t`（永远回绕）对齐。

💡 **补给：to_le_bytes。** 把 u64 拆成 8 个字节，低位在前（小端序）。WireGuard 密钥的字节序约定就是小端，CPU、GPU、`wg` 命令三方必须一致，4.7 节还会遇到。

下一节开始写域运算 `fe.rs`。
