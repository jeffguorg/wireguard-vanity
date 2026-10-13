# 6.2 splitmix64 与 clamp 的逐位复刻

这节写私钥派生的基础：给定 seed 和 gid，确定性地产出一个已 clamp 的候选私钥。kernel 侧与宿主端 `keygen.rs` 必须**逐位一致**，否则自检（7.4）对不上。

## 派生方案

GPU 上不能调系统随机数（也没必要）。设计是：

```text
work-item gid 的状态 = seed + gid × φ
```

φ = 0x9E3779B97F4A7C15 是黄金比例常数（奇数）。因为 φ 是奇数，`gid × φ mod 2⁶⁴` 对不同 gid 互不重复，每个 work-item 拿到不同的 splitmix64 初始状态，各取 4 个 u64 拼成 32 字节私钥。

## kernel 侧代码

在 `kernel.cl` 的常量区后面接上 splitmix64（文件中它在域运算之前）：

```c
// ---------------- splitmix64 私钥派生 ----------------

static u64 sm64_next(u64 *s) {
    u64 z = (*s += 0x9E3779B97F4A7C15UL);
    z = (z ^ (z >> 30)) * 0xBF58476D1CE4E5B9UL;
    z = (z ^ (z >> 27)) * 0x94D049BB133111EBUL;
    return z ^ (z >> 31);
}
```

> 💡 补给：C 里的 `u64 *s` 是「指向 u64 的指针」，约等于 Rust 的 `&mut u64`：`*s` 是解引用（读写指向的那个值），`&s` 是取地址。`sm64_next(&s)` 就相当于传一个可变借用进去，函数里的 `*s += ...` 改的是调用者的变量。

kernel 侧的 `derive_key`（调用 sm64_next 拼出私钥的那个函数）在文件中位于 `wg_pubkey` 之后，本节先不敲，6.4 节写到那里时再接上。

## 宿主端对照：keygen.rs

新建 `src/keygen.rs`，完整内容如下。同时在 `main.rs` 顶部加一行 `mod keygen;`（让编译器收录这个模块，它的测试才会跑）：

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::x25519;

    /// 派生出的私钥必须能通过 x25519_base 正常计算（冒烟测试）。
    #[test]
    fn derived_keys_work() {
        for gid in 0..3u64 {
            let key = derive_key(42, gid);
            let _ = x25519::x25519_base(&key);
        }
    }
}
```

对照两个版本：Rust 里要显式写 `wrapping_add`/`wrapping_mul`（防 debug 模式溢出 panic），C 里无符号数天然回绕所以直接 `+` `*`——逻辑逐位相同。

> 💡 补给：`x.to_le_bytes()` 把 u64 拆成 8 个字节（小端）。`copy_from_slice` 把这 8 字节拷进 `key` 的对应切片。`w.iter().enumerate()` 是「迭代器 + 序号」的标准写法。

敲完跑一下：`cargo test` 应该变成 4 个测试全过（新增 `keygen::tests::derived_keys_work`）。

> 💡 补给：cargo 输出里**黄色 warning ≠ 红色 error**。这会你可能看到（`cargo build` 时）`derive_key`、`splitmix64` 等函数「never used」的警告——是暂时的，第 7 章的 main 会用上它们。只要 `cargo test` 全绿就算过关。

## 小结

私钥派生两侧一致：seed + gid·φ → splitmix64 ×4 → clamp。下一节移植 fe.rs 的整套域运算。
