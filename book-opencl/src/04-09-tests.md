# 4.9 单元测试：RFC 7748 与 dalek 对拍

这节给 `x25519.rs` 补上测试模块并跑通。三层验证的第一层（RFC 官方向量 + 成熟库对拍）就在这里建立。

## 测试代码

在 `src/x25519.rs` 末尾追加：

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

三个测试的分工：

1. **`rfc7748_vector`**：RFC 7748 §5.2 的官方测试向量，一组输入输出钉死正确性；
2. **`rfc7748_iteration`**：把输出当输入迭代 1000 次再对答案——单次算对可能靠运气，迭代 1000 次会把任何微小错误放大到必然暴露；
3. **`matches_dalek`**：用 splitmix64 伪随机生成 100 个标量，我们的实现和 curve25519-dalek 各算一遍，逐字节必须相等。

> 💡 补给：`#[cfg(test)]` 表示这个模块只在 `cargo test` 时编译，正式构建不包含它。`#[test]` 标记的函数会被测试框架自动发现并逐个运行。`assert_eq!(a, b)` 不等就 panic、测试失败。

> 💡 补给：`matches_dalek` 里那段「异或-移位-乘大常数」就是 splitmix64 伪随机数生成器——第 6 章会把它原样搬上 GPU 当私钥派生器，到时会再见一次。

## 跑起来

```console
$ cargo test
   Compiling wg-vanity-opencl v0.1.0 ...
    Finished `test` profile [unoptimized + debuginfo] target(s) ...

running 3 tests
test x25519::tests::rfc7748_vector ... ok
test x25519::tests::rfc7748_iteration ... ok
test x25519::tests::matches_dalek ... ok

test result: ok. 3 passed; 0 failed
```

**3 passed** 就是你的 CPU 参考实现被官方向量和成熟库双重认可的证明。如果有红的，先检查 4.4 的 mul 有没有抄错行——竖式是最容易抄串行的部分。

## 小结

CPU 参考实现完成且可信。下一章补 OpenCL 的概念课，第 6 章把这套代码搬上 GPU。
