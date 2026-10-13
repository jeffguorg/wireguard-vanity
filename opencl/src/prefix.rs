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
