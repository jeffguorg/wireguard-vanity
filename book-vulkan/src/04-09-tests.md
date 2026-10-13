# 4.9 单元测试：RFC 7748 向量 + dalek 对拍

这节给 `x25519.rs` 和 `keygen.rs` 补上测试模块，然后跑通全套 `cargo test`。从此 CPU 参考实现就是「可信的」，第 6 章的 GLSL 翻译以它为准。

## x25519.rs 的测试

接在 `x25519_impl` 之后：

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn from_hex(s: &str) -> [u8; 32] {
        let mut out = [0u8; 32];
        for i in 0..32 {
            out[i] = u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap();
        }
        out
    }

    /// RFC 7748 §5.2 的 X25519 测试向量。
    #[test]
    fn rfc7748_vector() {
        let scalar = from_hex("77076d0a7318a57d3c16c17251b26645df4c2f87ebc0992ab177fba51db92c2a");
        let expect = from_hex("8520f0098930a754748b7ddcb43ef75a0dbf3a0d26381af4eba4a98eaa9b4e6a");
        assert_eq!(x25519_base(&scalar), expect);
    }

    /// RFC 7748 的迭代向量：k = u = 9 迭代 1 次 / 1000 次。
    #[test]
    fn rfc7748_iteration() {
        let nine = || {
            let mut b = [0u8; 32];
            b[0] = 9;
            b
        };
        let mut k = nine();
        let mut u = nine();
        for _ in 0..1 {
            let (new_k, new_u) = (x25519_with_u(&k, &u), k);
            k = new_k;
            u = new_u;
        }
        assert_eq!(
            k,
            from_hex("422c8e7a6227d7bca1350b3e2bb7279f7897b87bb6854b783c60e80311ae3079")
        );

        let mut k = nine();
        let mut u = nine();
        for _ in 0..1000 {
            let (new_k, new_u) = (x25519_with_u(&k, &u), k);
            k = new_k;
            u = new_u;
        }
        assert_eq!(
            k,
            from_hex("684cf59ba83309552800ef566f2f4d3c1c3887c49360e3875f2eb94d99532c51")
        );
    }

    /// 与 curve25519-dalek 对拍（确定性伪随机标量）。
    #[test]
    fn matches_dalek() {
        let mut s: u64 = 0x1234_5678_9abc_def0;
        for _ in 0..100 {
            let mut key = [0u8; 32];
            for chunk in key.chunks_exact_mut(8) {
                s = s.wrapping_add(0x9E37_79B9_7F4A_7C15);
                let mut z = s;
                z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
                z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
                z ^= z >> 31;
                chunk.copy_from_slice(&z.to_le_bytes());
            }
            let ours = x25519_base(&key);
            let theirs = curve25519_dalek::constants::X25519_BASEPOINT
                .mul_clamped(key)
                .to_bytes();
            assert_eq!(ours, theirs);
        }
    }
}
```


## 补上 keygen.rs 的测试

4.1 节写 `keygen.rs` 时把测试模块留到了现在（它要调用 `x25519_base`，4.1 时还没有）——与 prefix.rs 的测试推迟到 7.7 节是同一模式。追加到 `src/keygen.rs` 末尾：

```rust
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
## 三层防线

1. **RFC 7748 单向量**：官方给的「输入私钥 → 输出公钥」，错了说明实现有硬错误；
2. **RFC 7748 迭代向量**：k、u 都初始为 9，反复 `k, u = X25519(k, u), k` 迭代 1000 次——任何微小的进位/折叠错误都会被迭代放大，藏不住；
3. **dalek 对拍**：`curve25519-dalek` 是生产级库，用它当裁判随机对拍 100 个私钥（私钥由 splitmix64 确定性生成，测试可复现）。注意 `mul_clamped`：dalek 内部会做 clamp，所以我们传未 clamp 的原始 32 字节。

## 跑起来

```console
$ cargo test
running 4 tests
test keygen::tests::derived_keys_work ... ok
test x25519::tests::rfc7748_vector ... ok
test x25519::tests::rfc7748_iteration ... ok
test x25519::tests::matches_dalek ... ok

test result: ok. 4 passed; 0 failed
```

4 个测试全绿：x25519 的 3 个 + keygen 的 1 个（本节刚补的冒烟测试）。如果哪个红了，回去逐行对照第 4 章的代码块——99% 是敲错了系数或循环范围。

💡 **补给：测试也可能编译警告。**（如果看到 dead_code 警告，无害，第 7 章写完 main 就消失。）
