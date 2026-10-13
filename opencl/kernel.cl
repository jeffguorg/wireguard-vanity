// WireGuard 公钥前缀暴力搜索 kernel。
//
// 与宿主端 Rust 代码（src/fe.rs, src/x25519.rs, src/keygen.rs）一一对应，
// 用 OpenCL C 的 ulong 重写同一套 radix-2^25.5 域运算。

typedef ulong u64;
typedef uint  u32;
typedef uchar u8;

typedef struct { u64 v[10]; } fe;

#define M26 ((u64)0x3ffffff)
#define M25 ((u64)0x1ffffff)

// ---------------- splitmix64 私钥派生 ----------------

static u64 sm64_next(u64 *s) {
    u64 z = (*s += 0x9E3779B97F4A7C15UL);
    z = (z ^ (z >> 30)) * 0xBF58476D1CE4E5B9UL;
    z = (z ^ (z >> 27)) * 0x94D049BB133111EBUL;
    return z ^ (z >> 31);
}

// ---------------- 域运算 ----------------

static fe fe_carry(fe h) {
    u64 c;
    c = h.v[0] >> 26; h.v[1] += c; h.v[0] &= M26;
    c = h.v[1] >> 25; h.v[2] += c; h.v[1] &= M25;
    c = h.v[2] >> 26; h.v[3] += c; h.v[2] &= M26;
    c = h.v[3] >> 25; h.v[4] += c; h.v[3] &= M25;
    c = h.v[4] >> 26; h.v[5] += c; h.v[4] &= M26;
    c = h.v[5] >> 25; h.v[6] += c; h.v[5] &= M25;
    c = h.v[6] >> 26; h.v[7] += c; h.v[6] &= M26;
    c = h.v[7] >> 25; h.v[8] += c; h.v[7] &= M25;
    c = h.v[8] >> 26; h.v[9] += c; h.v[8] &= M26;
    c = h.v[9] >> 25; h.v[0] += 19 * c; h.v[9] &= M25;
    c = h.v[0] >> 26; h.v[1] += c; h.v[0] &= M26;
    return h;
}

static fe fe_add(fe a, fe b) {
    fe h;
    for (int i = 0; i < 10; i++) h.v[i] = a.v[i] + b.v[i];
    return h;
}

// a + 2p - b（加 2p 保证无负数）
static fe fe_sub(fe a, fe b) {
    const u64 two_p[10] = {
        2UL * ((1UL << 26) - 19),
        2UL * ((1UL << 25) - 1), 2UL * ((1UL << 26) - 1),
        2UL * ((1UL << 25) - 1), 2UL * ((1UL << 26) - 1),
        2UL * ((1UL << 25) - 1), 2UL * ((1UL << 26) - 1),
        2UL * ((1UL << 25) - 1), 2UL * ((1UL << 26) - 1),
        2UL * ((1UL << 25) - 1),
    };
    fe h;
    for (int i = 0; i < 10; i++) h.v[i] = a.v[i] + two_p[i] - b.v[i];
    return h;
}

static fe fe_mul(fe f, fe g) {
    u64 f0=f.v[0], f1=f.v[1], f2=f.v[2], f3=f.v[3], f4=f.v[4];
    u64 f5=f.v[5], f6=f.v[6], f7=f.v[7], f8=f.v[8], f9=f.v[9];
    u64 g0=g.v[0], g1=g.v[1], g2=g.v[2], g3=g.v[3], g4=g.v[4];
    u64 g5=g.v[5], g6=g.v[6], g7=g.v[7], g8=g.v[8], g9=g.v[9];

    u64 f1_2 = 2*f1, f3_2 = 2*f3, f5_2 = 2*f5, f7_2 = 2*f7, f9_2 = 2*f9;
    u64 g1_19 = 19*g1, g2_19 = 19*g2, g3_19 = 19*g3, g4_19 = 19*g4, g5_19 = 19*g5;
    u64 g6_19 = 19*g6, g7_19 = 19*g7, g8_19 = 19*g8, g9_19 = 19*g9;

    u64 h0 = f0*g0 + f1_2*g9_19 + f2*g8_19 + f3_2*g7_19 + f4*g6_19
           + f5_2*g5_19 + f6*g4_19 + f7_2*g3_19 + f8*g2_19 + f9_2*g1_19;
    u64 h1 = f0*g1 + f1*g0 + f2*g9_19 + f3*g8_19 + f4*g7_19
           + f5*g6_19 + f6*g5_19 + f7*g4_19 + f8*g3_19 + f9*g2_19;
    u64 h2 = f0*g2 + f1_2*g1 + f2*g0 + f3_2*g9_19 + f4*g8_19
           + f5_2*g7_19 + f6*g6_19 + f7_2*g5_19 + f8*g4_19 + f9_2*g3_19;
    u64 h3 = f0*g3 + f1*g2 + f2*g1 + f3*g0 + f4*g9_19
           + f5*g8_19 + f6*g7_19 + f7*g6_19 + f8*g5_19 + f9*g4_19;
    u64 h4 = f0*g4 + f1_2*g3 + f2*g2 + f3_2*g1 + f4*g0
           + f5_2*g9_19 + f6*g8_19 + f7_2*g7_19 + f8*g6_19 + f9_2*g5_19;
    u64 h5 = f0*g5 + f1*g4 + f2*g3 + f3*g2 + f4*g1 + f5*g0
           + f6*g9_19 + f7*g8_19 + f8*g7_19 + f9*g6_19;
    u64 h6 = f0*g6 + f1_2*g5 + f2*g4 + f3_2*g3 + f4*g2 + f5_2*g1 + f6*g0
           + f7_2*g9_19 + f8*g8_19 + f9_2*g7_19;
    u64 h7 = f0*g7 + f1*g6 + f2*g5 + f3*g4 + f4*g3 + f5*g2 + f6*g1 + f7*g0
           + f8*g9_19 + f9*g8_19;
    u64 h8 = f0*g8 + f1_2*g7 + f2*g6 + f3_2*g5 + f4*g4
           + f5_2*g3 + f6*g2 + f7_2*g1 + f8*g0 + f9_2*g9_19;
    u64 h9 = f0*g9 + f1*g8 + f2*g7 + f3*g6 + f4*g5
           + f5*g4 + f6*g3 + f7*g2 + f8*g1 + f9*g0;

    fe h = {{ h0, h1, h2, h3, h4, h5, h6, h7, h8, h9 }};
    return fe_carry(h);
}

static fe fe_sq(fe f) {
    u64 f0=f.v[0], f1=f.v[1], f2=f.v[2], f3=f.v[3], f4=f.v[4];
    u64 f5=f.v[5], f6=f.v[6], f7=f.v[7], f8=f.v[8], f9=f.v[9];

    u64 f0_2 = 2*f0, f1_2 = 2*f1, f2_2 = 2*f2, f3_2 = 2*f3, f4_2 = 2*f4;
    u64 f5_2 = 2*f5, f6_2 = 2*f6, f7_2 = 2*f7;
    u64 f5_38 = 38*f5, f6_19 = 19*f6, f7_38 = 38*f7, f8_19 = 19*f8, f9_38 = 38*f9;

    u64 h0 = f0*f0 + f1_2*f9_38 + f2_2*f8_19 + f3_2*f7_38 + f4_2*f6_19 + f5*f5_38;
    u64 h1 = f0_2*f1 + f2*f9_38 + f3_2*f8_19 + f4*f7_38 + f5_2*f6_19;
    u64 h2 = f0_2*f2 + f1_2*f1 + f3_2*f9_38 + f4_2*f8_19 + f5_2*f7_38 + f6*f6_19;
    u64 h3 = f0_2*f3 + f1_2*f2 + f4*f9_38 + f5_2*f8_19 + f6*f7_38;
    u64 h4 = f0_2*f4 + f1_2*f3_2 + f2*f2 + f5_2*f9_38 + f6_2*f8_19 + f7*f7_38;
    u64 h5 = f0_2*f5 + f1_2*f4 + f2_2*f3 + f6*f9_38 + f7_2*f8_19;
    u64 h6 = f0_2*f6 + f1_2*f5_2 + f2_2*f4 + f3_2*f3 + f7_2*f9_38 + f8*f8_19;
    u64 h7 = f0_2*f7 + f1_2*f6 + f2_2*f5 + f3_2*f4 + f8*f9_38;
    u64 h8 = f0_2*f8 + f1_2*f7_2 + f2_2*f6 + f3_2*f5_2 + f4*f4 + f9*f9_38;
    u64 h9 = f0_2*f9 + f1_2*f8 + f2_2*f7 + f3_2*f6 + f4_2*f5;

    fe h = {{ h0, h1, h2, h3, h4, h5, h6, h7, h8, h9 }};
    return fe_carry(h);
}

static fe fe_mul121665(fe f) {
    fe h;
    for (int i = 0; i < 10; i++) h.v[i] = f.v[i] * 121665;
    return h;
}

// z^(2^255 - 21)，ref10 加法链：254 次平方 + 11 次乘法
static fe fe_invert(fe z) {
    fe t0 = fe_sq(z);
    fe t1 = fe_sq(t0);
    t1 = fe_sq(t1);
    t1 = fe_mul(z, t1);
    t0 = fe_mul(t0, t1);
    fe t2 = fe_sq(t0);
    t1 = fe_mul(t1, t2);

    t2 = fe_sq(t1);
    for (int i = 1; i < 5; i++) t2 = fe_sq(t2);
    t1 = fe_mul(t2, t1);

    t2 = fe_sq(t1);
    for (int i = 1; i < 10; i++) t2 = fe_sq(t2);
    t2 = fe_mul(t2, t1);

    fe t3 = fe_sq(t2);
    for (int i = 1; i < 20; i++) t3 = fe_sq(t3);
    t2 = fe_mul(t3, t2);

    t2 = fe_sq(t2);
    for (int i = 1; i < 10; i++) t2 = fe_sq(t2);
    t1 = fe_mul(t2, t1);

    t2 = fe_sq(t1);
    for (int i = 1; i < 50; i++) t2 = fe_sq(t2);
    t2 = fe_mul(t2, t1);

    t3 = fe_sq(t2);
    for (int i = 1; i < 100; i++) t3 = fe_sq(t3);
    t2 = fe_mul(t3, t2);

    t2 = fe_sq(t2);
    for (int i = 1; i < 50; i++) t2 = fe_sq(t2);
    t1 = fe_mul(t2, t1);

    t1 = fe_sq(t1);
    for (int i = 1; i < 5; i++) t1 = fe_sq(t1);
    return fe_mul(t1, t0);
}

static void fe_tobytes(fe h, u8 *s) {
    h = fe_carry(h);

    u64 q = (19 * h.v[9] + ((u64)1 << 24)) >> 25;
    q = (h.v[0] + q) >> 26;
    q = (h.v[1] + q) >> 25;
    q = (h.v[2] + q) >> 26;
    q = (h.v[3] + q) >> 25;
    q = (h.v[4] + q) >> 26;
    q = (h.v[5] + q) >> 25;
    q = (h.v[6] + q) >> 26;
    q = (h.v[7] + q) >> 25;
    q = (h.v[8] + q) >> 26;
    q = (h.v[9] + q) >> 25;

    h.v[0] += 19 * q;
    // 最终进位链：最高 limb 溢出的位代表 q·2^255，直接丢弃（不能再 ×19 折回）
    u64 c;
    c = h.v[0] >> 26; h.v[1] += c; h.v[0] &= M26;
    c = h.v[1] >> 25; h.v[2] += c; h.v[1] &= M25;
    c = h.v[2] >> 26; h.v[3] += c; h.v[2] &= M26;
    c = h.v[3] >> 25; h.v[4] += c; h.v[3] &= M25;
    c = h.v[4] >> 26; h.v[5] += c; h.v[4] &= M26;
    c = h.v[5] >> 25; h.v[6] += c; h.v[5] &= M25;
    c = h.v[6] >> 26; h.v[7] += c; h.v[6] &= M26;
    c = h.v[7] >> 25; h.v[8] += c; h.v[7] &= M25;
    c = h.v[8] >> 26; h.v[9] += c; h.v[8] &= M26;
    h.v[9] &= M25;

    u64 v0=h.v[0], v1=h.v[1], v2=h.v[2], v3=h.v[3], v4=h.v[4];
    u64 v5=h.v[5], v6=h.v[6], v7=h.v[7], v8=h.v[8], v9=h.v[9];
    s[0]  = (u8)(v0);
    s[1]  = (u8)(v0 >> 8);
    s[2]  = (u8)(v0 >> 16);
    s[3]  = (u8)((v0 >> 24) | (v1 << 2));
    s[4]  = (u8)(v1 >> 6);
    s[5]  = (u8)(v1 >> 14);
    s[6]  = (u8)((v1 >> 22) | (v2 << 3));
    s[7]  = (u8)(v2 >> 5);
    s[8]  = (u8)(v2 >> 13);
    s[9]  = (u8)((v2 >> 21) | (v3 << 5));
    s[10] = (u8)(v3 >> 3);
    s[11] = (u8)(v3 >> 11);
    s[12] = (u8)((v3 >> 19) | (v4 << 6));
    s[13] = (u8)(v4 >> 2);
    s[14] = (u8)(v4 >> 10);
    s[15] = (u8)(v4 >> 18);
    s[16] = (u8)(v5);
    s[17] = (u8)(v5 >> 8);
    s[18] = (u8)(v5 >> 16);
    s[19] = (u8)((v5 >> 24) | (v6 << 1));
    s[20] = (u8)(v6 >> 7);
    s[21] = (u8)(v6 >> 15);
    s[22] = (u8)((v6 >> 23) | (v7 << 3));
    s[23] = (u8)(v7 >> 5);
    s[24] = (u8)(v7 >> 13);
    s[25] = (u8)((v7 >> 21) | (v8 << 4));
    s[26] = (u8)(v8 >> 4);
    s[27] = (u8)(v8 >> 12);
    s[28] = (u8)((v8 >> 20) | (v9 << 6));
    s[29] = (u8)(v9 >> 2);
    s[30] = (u8)(v9 >> 10);
    s[31] = (u8)(v9 >> 18);
}

// ---------------- X25519 ----------------

static void cswap(u64 swap, fe *a, fe *b) {
    u64 mask = (u64)0 - swap;
    for (int i = 0; i < 10; i++) {
        u64 t = mask & (a->v[i] ^ b->v[i]);
        a->v[i] ^= t;
        b->v[i] ^= t;
    }
}

// 公钥 = X25519(w, 基点 9) 的 32 字节小端序
static void wg_pubkey(const u64 *w, u8 *out) {
    fe x1 = {{ 9, 0, 0, 0, 0, 0, 0, 0, 0, 0 }};
    fe x2 = {{ 1, 0, 0, 0, 0, 0, 0, 0, 0, 0 }};
    fe z2 = {{ 0 }};
    fe x3 = x1;
    fe z3 = {{ 1, 0, 0, 0, 0, 0, 0, 0, 0, 0 }};

    u64 swap = 0;
    for (int t = 254; t >= 0; t--) {
        u64 bit = (w[t >> 6] >> (t & 63)) & 1;
        swap ^= bit;
        cswap(swap, &x2, &x3);
        cswap(swap, &z2, &z3);
        swap = bit;

        fe a  = fe_add(x2, z2);
        fe aa = fe_sq(a);
        fe b  = fe_sub(x2, z2);
        fe bb = fe_sq(b);
        fe e  = fe_sub(aa, bb);
        fe c  = fe_add(x3, z3);
        fe d  = fe_sub(x3, z3);
        fe da = fe_mul(d, a);
        fe cb = fe_mul(c, b);
        fe dac = fe_add(da, cb);
        fe dmc = fe_sub(da, cb);

        x3 = fe_sq(dac);
        z3 = fe_mul(x1, fe_sq(dmc));
        x2 = fe_mul(aa, bb);
        z2 = fe_mul(e, fe_add(aa, fe_carry(fe_mul121665(e))));
    }
    cswap(swap, &x2, &x3);
    cswap(swap, &z2, &z3);

    fe r = fe_mul(x2, fe_invert(z2));
    fe_tobytes(r, out);
}

// 派生第 gid 个候选私钥（已 clamp），与宿主端 keygen.rs 逐位一致
static void derive_key(u64 seed, u64 gid, u64 *w) {
    u64 s = seed + gid * 0x9E3779B97F4A7C15UL;
    w[0] = sm64_next(&s);
    w[1] = sm64_next(&s);
    w[2] = sm64_next(&s);
    w[3] = sm64_next(&s);
    w[0] &= ~7UL;
    w[3] = (w[3] & 0x7FFFFFFFFFFFFFFFUL) | 0x4000000000000000UL;
}

// ---------------- 入口 ----------------

// 前缀搜索：命中则把私钥（4 个 u64）写入 keys[slot*4 .. slot*4+3]
__kernel void vanity(u64 seed,
                     __constant const u8 *pat,
                     __constant const u8 *mask,
                     u32 patlen,
                     __global u32 *found,
                     __global u64 *keys) {
    u64 gid = (u64)get_global_id(0);
    u64 w[4];
    derive_key(seed, gid, w);

    u8 pub[32];
    wg_pubkey(w, pub);

    for (u32 i = 0; i < patlen; i++) {
        if ((pub[i] & mask[i]) != pat[i]) return;
    }

    u32 slot = atomic_inc((volatile __global u32 *)found);
    if (slot < 64) {
        keys[slot * 4 + 0] = w[0];
        keys[slot * 4 + 1] = w[1];
        keys[slot * 4 + 2] = w[2];
        keys[slot * 4 + 3] = w[3];
    }
}

// 自检：把每个候选的公钥原样写出，宿主端与参考实现比对
__kernel void selftest_keys(u64 seed, __global u8 *pubs) {
    u64 gid = (u64)get_global_id(0);
    u64 w[4];
    derive_key(seed, gid, w);
    u8 pub[32];
    wg_pubkey(w, pub);
    for (int i = 0; i < 32; i++) pubs[gid * 32 + i] = pub[i];
}
