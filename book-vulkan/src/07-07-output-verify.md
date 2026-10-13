# 7.7 输出与 wg pubkey 交叉验证

这节收尾：补上 main 末尾的统计、prefix.rs 的测试模块，然后用官方 `wg` 命令做端到端验证。

## main 的最后几行

接在 7.5 节的主循环之后：

```rust
    let secs = t_start.elapsed().as_secs_f64();
    eprintln!(
        "完成：共尝试 {} 次，耗时 {:.1}s，平均 {:.1} M/s",
        total,
        secs,
        total as f64 / secs / 1e6
    );
    Ok(())
}
```

`main` 返回 `Ok(())` 时进程正常退出；任何一步的 `Err` 都会被 Rust 运行时打印出来并以非零码退出。

## 补上 prefix.rs 的测试

6.2 节写 `src/prefix.rs` 时留了尾巴，现在把测试模块追加到文件末尾：

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use base64::{engine::general_purpose::STANDARD, Engine as _};

    #[test]
    fn matches_full_bytes() {
        let pubkey = [0x85u8, 0x20, 0xf0, 0x09, 0x89, 0x30, 0xa7, 0x54,
                      0x74, 0x8b, 0x7d, 0xdc, 0xb4, 0x3e, 0xf7, 0x5a,
                      0x0d, 0xbf, 0x3a, 0x0d, 0x26, 0x38, 0x1a, 0xf4,
                      0xeb, 0xa4, 0xa9, 0x8e, 0xaa, 0x9b, 0x4e, 0x6a];
        let b64 = STANDARD.encode(pubkey); // "hSDw..." 
        for n in 1..=20 {
            let p = from_prefix(&b64[..n]).unwrap();
            assert!(p.matches(&pubkey), "前缀 {:?} 应匹配", &b64[..n]);
            // 把前缀最后一个字符换掉，通常应不再匹配（构造一个必然不同的）
            let mut bad = pubkey;
            bad[0] ^= 0x01;
            if n > 1 {
                assert!(!p.matches(&bad));
            }
        }
    }

    #[test]
    fn rejects_bad_input() {
        assert!(from_prefix("").is_err());
        assert!(from_prefix(&"A".repeat(44)).is_err());
        assert!(from_prefix("ab= ").is_err());
    }
}
```

用 RFC 7748 向量那个公钥（base64 以 `hSDw` 开头）测 1~20 字符的前缀都能匹配、翻转一字节后不再匹配；另测三类非法输入。跑一次全套：

```console
$ cargo test
running 6 tests
...
test result: ok. 6 passed; 0 failed
```

## 端到端：跑一次真搜索

```console
$ cargo run --release -- hSD
使用设备: AMD Radeon RX 7900 XTX (RADV NAVI31, LLVM ...)
前缀 "hSD"：期望尝试次数 ≈ 2^18.0
私钥: <base64 私钥>
公钥: hSD...（44 字符）

完成：共尝试 ... 次，耗时 ...s，平均 4.9 M/s
```

## wg pubkey 交叉验证

最后一道保险——用 WireGuard 官方工具验证私钥是真的：

```console
$ echo '<刚打印的私钥>' | wg pubkey
```

输出的公钥应该和程序打印的**逐字符一致**。一致意味着：我们的 CPU 参考实现、GPU shader、官方实现三方独立同意——这个私钥可以真的拿去配 WireGuard。

至此 `wg-vanity-vulkan` 全部完成：`cargo test` 6 项全绿、`--selftest` GPU 对拍通过、搜索结果经官方工具验证。下一章看性能数据和调优方向。
