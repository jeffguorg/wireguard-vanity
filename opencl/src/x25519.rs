//! X25519（RFC 7748）： Montgomery 梯子 + 域求逆，只支持基点（u=9）。
//!
//! WireGuard 只需要 `X25519(私钥, 基点)`，所以不需要处理任意 u 坐标。

use crate::fe::{self, Fe};

/// 常数时间条件交换：swap=1 时交换 a、b，swap=0 时不变。
/// 用掩码而非分支实现（GPU 上也便宜）。
fn cswap(swap: u64, a: &mut Fe, b: &mut Fe) {
    let mask = 0u64.wrapping_sub(swap);
    for i in 0..10 {
        let t = mask & (a.0[i] ^ b.0[i]);
        a.0[i] ^= t;
        b.0[i] ^= t;
    }
}

/// 对 32 字节私钥做 RFC 7748 规定的 clamp。
pub fn clamp(k: &mut [u8; 32]) {
    k[0] &= 248;
    k[31] &= 127;
    k[31] |= 64;
}

/// X25519(scalar, u)，通用形式。
pub fn x25519_with_u(scalar: &[u8; 32], u: &[u8; 32]) -> [u8; 32] {
    x25519_impl(scalar, &fe::from_bytes(u))
}

/// X25519(scalar, 基点 9)，返回公钥的 32 字节小端序。
pub fn x25519_base(scalar: &[u8; 32]) -> [u8; 32] {
    // 基点 u 坐标 = 9
    x25519_impl(scalar, &Fe([9, 0, 0, 0, 0, 0, 0, 0, 0, 0]))
}

fn x25519_impl(scalar: &[u8; 32], x1: &Fe) -> [u8; 32] {
    let mut s = *scalar;
    clamp(&mut s);

    let x1 = *x1;
    let mut x2 = Fe::ONE;
    let mut z2 = Fe::ZERO;
    let mut x3 = x1;
    let mut z3 = Fe::ONE;

    let mut swap: u64 = 0;
    // 从高到低扫描 scalar 的 254..=0 位
    for t in (0..255).rev() {
        let bit = ((s[t / 8] >> (t & 7)) & 1) as u64;
        swap ^= bit;
        cswap(swap, &mut x2, &mut x3);
        cswap(swap, &mut z2, &mut z3);
        swap = bit;

        let a = fe::add(&x2, &z2);
        let aa = fe::sq(&a);
        let b = fe::sub(&x2, &z2);
        let bb = fe::sq(&b);
        let e = fe::sub(&aa, &bb);
        let c = fe::add(&x3, &z3);
        let d = fe::sub(&x3, &z3);
        let da = fe::mul(&d, &a);
        let cb = fe::mul(&c, &b);
        let da_plus_cb = fe::add(&da, &cb);
        let da_minus_cb = fe::sub(&da, &cb);

        x3 = fe::sq(&da_plus_cb);
        z3 = fe::mul(&x1, &fe::sq(&da_minus_cb));
        x2 = fe::mul(&aa, &bb);
        // z2 = E * (AA + a24 * E)，mul121665 的结果 limb 较大，先归约再参与运算
        z2 = fe::mul(&e, &fe::add(&aa, &fe::carry(&fe::mul121665(&e))));
    }
    cswap(swap, &mut x2, &mut x3);
    cswap(swap, &mut z2, &mut z3);

    // u = x2 / z2 = x2 * z2^(p-2)
    fe::to_bytes(&fe::mul(&x2, &fe::invert(&z2)))
}

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

