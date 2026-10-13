# 6.5 前缀匹配：为什么不在 GPU 上做 base64

这节写宿主端最后一个模块 `src/prefix.rs`，并解释 kernel 里那三行匹配代码的原理：把 base64 前缀**预编译**成字节模式 + 掩码，GPU 只需比字节。

## 为什么不在 GPU 上编码 base64

每个候选都要比前缀。如果在 GPU 上把 32 字节公钥编成 44 字符再 strcmp：每个 work-item 多做一次编码（查表、拼位），400 万次/轮全是浪费。更好的办法：base64 编码是**确定性的位重排**，「公钥 base64 以 P 开头」完全等价于「公钥前若干字节满足某个位模式」。宿主侧只算一次：

- **pat**（模式）：前缀对应的字节内容；
- **mask**（掩码）：每个字节里哪些位参与比较（前缀末尾不满 8 bit 时，最后一字节只比高位）。

kernel 里的匹配循环因此只要三行（下一节写进 `vanity`）：

```c
    for (u32 i = 0; i < patlen; i++) {
        if ((pub[i] & mask[i]) != pat[i]) return;
    }
```

## prefix.rs 完整代码

新建 `src/prefix.rs`：

```rust
//! 把 base64 前缀预编译为「字节模式 + 掩码」。
//!
//! GPU kernel 里不做 base64 编码：公钥的 32 字节就是 base64 的输入，
//! 所以前缀匹配可以直接翻译成对公钥前若干字节的比较。
//! base64 每字符 6 bit，按 MSB-first 拼进字节流；当前缀比特数不是 8 的
//! 整数倍时，最后一个字节只比较高位的几 bit（用 mask 实现）。

/// base64 字符 → 6 bit 值
fn b64_value(c: u8) -> Option<u8> {
    match c {
        b'A'..=b'Z' => Some(c - b'A'),
        b'a'..=b'z' => Some(c - b'a' + 26),
        b'0'..=b'9' => Some(c - b'0' + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    }
}

pub struct Pattern {
    /// 期望的字节模式
    pub pat: [u8; 44],
    /// 每个字节参与比较的位（最后一个字节可能不满 8 位）
    pub mask: [u8; 44],
    /// 需要比较的字节数
    pub len: usize,
}

impl Pattern {
    /// 匹配判断，与 GPU kernel 中的逻辑一致。
    pub fn matches(&self, pubkey: &[u8; 32]) -> bool {
        for i in 0..self.len {
            if pubkey[i] & self.mask[i] != self.pat[i] {
                return false;
            }
        }
        true
    }
}

pub fn from_prefix(prefix: &str) -> Result<Pattern, String> {
    if prefix.is_empty() || prefix.len() > 42 {
        return Err(format!(
            "前缀长度需在 1..=42 之间（43 个字符需要 33 字节的比较窗口，已超出公钥的 32 字节），当前 {}",
            prefix.len()
        ));
    }

    let mut pat = [0u8; 44];
    let mut mask = [0u8; 44];
    let mut len = 0usize;

    let mut acc: u32 = 0; // 暂存未凑满一字节的比特
    let mut nbits: u32 = 0; // acc 中的有效比特数
    for &c in prefix.as_bytes() {
        let v = b64_value(c).ok_or_else(|| format!("非法 base64 字符: {:?}", c as char))?;
        acc = (acc << 6) | v as u32;
        nbits += 6;
        if nbits >= 8 {
            nbits -= 8;
            pat[len] = (acc >> nbits) as u8;
            mask[len] = 0xFF;
            len += 1;
            acc &= (1 << nbits) - 1;
        }
    }
    if nbits > 0 {
        // 不足 8 bit 的尾巴：左对齐，mask 只保留高 nbits 位
        pat[len] = (acc << (8 - nbits)) as u8;
        mask[len] = 0xFFu8 << (8 - nbits);
        len += 1;
    }

    Ok(Pattern { pat, mask, len })
}

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
            // 翻转公钥第 0 字节的最低位：n>1 时该字节掩码为 0xFF，必然不再匹配
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
        assert!(from_prefix(&"A".repeat(43)).is_err());
        assert!(from_prefix(&"A".repeat(44)).is_err());
        assert!(from_prefix("ab= ").is_err());
    }
}
```

`from_prefix` 的核心是个「位流拼字节」循环：`acc` 像传送带，每来一个字符左移 6 位塞进去，攒够 8 位就吐出一个字节；循环结束若还有零头（nbits > 0），左对齐进最后一字节、掩码只留高位。

> 💡 补给：`Option<u8>` 表示「可能有值」（`Some(v)` / `None`）；`ok_or_else(...)?` 把 `Option` 转成 `Result`，为 `None` 时返回错误。`b'A'` 是字节字面量（u8）。`match` 里的 `b'A'..=b'Z'` 是区间模式。

在 `main.rs` 顶部加上 `mod prefix;`，跑 `cargo test`——应该 6 个测试全过（新增 `prefix::tests` 两个）。

## 小结

匹配的热路径被压成「每候选几字节比较」，所有 base64 的复杂性都在宿主侧一次性消化。下节写 `vanity` 入口，把命中结果传回来。
