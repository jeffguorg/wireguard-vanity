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
