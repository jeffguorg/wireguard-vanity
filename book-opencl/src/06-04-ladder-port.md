# 6.4 梯子与求逆的移植

这节完成 kernel 的主体计算：`fe_invert`、`fe_tobytes`、`cswap`、`wg_pubkey`。全部与第 4 章逐行对应。

## fe_invert

接在 `fe_mul121665` 后面。和 4.6 节是同一条加法链，Rust 的 `for _ in 1..5` 变成 C 的 `for (int i = 1; i < 5; i++)`：

```c
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
```

## fe_tobytes

强归约 + 打包，对应 4.7 节的 `to_bytes`。区别在于输出用指针参数（写进调用者的数组）：

```c
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
```

这里的 `u8 *s` 没写地址空间修饰符，默认为 `__private`（本 work-item 的私有内存）——调用方 `wg_pubkey` 里的 `u8 pub[32]` 正是私有数组，类型匹配。

## cswap 与 wg_pubkey

梯子主体，对应 4.8 节：

```c
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
```

和 Rust 版的两处差异，都值得琢磨：

1. **取标量第 t 位**：Rust 版在字节数组上取位（`s[t/8] >> (t&7)`）；kernel 版直接在 4 个 u64 上取（`w[t>>6] >> (t&63)`，t/64 号字的第 t%64 位）——因为私钥在这边本来就是 u64 形式，省掉一次拆字节；
2. **clamp 不在梯子里**：kernel 的 clamp 在 `derive_key` 里做（宿主端版本 6.2 节已经写过，kernel 版本节末尾敲入），`wg_pubkey` 直接吃处理好的 u64。

## derive_key

文件中接下来是 `derive_key`——6.2 节已经写过它的宿主端孪生（keygen.rs），现在把 kernel 版敲进来，位置就接在上面 `wg_pubkey` 的 `}` 之后：

```c
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
```

最后两行就是 u64 形式的 clamp：`w[0] &= ~7` 清低 3 位（对应字节版 `k[0] &= 248`）；`w[3]` 清最高位、置位次高位（对应 `k[31] &= 127; k[31] |= 64`，注意小端序下 w[3] 的高字节就是 k[31]）。

再往文件后面是两个 `__kernel` 入口，下三节写。

> 💡 补给：`fe z2 = {{ 0 }};` 是 C 的零初始化惯例：只给第一个元素赋 0，其余自动为 0。`a->v[i]` 等价于 `(*a).v[i]`，通过指针取字段。

## 小结

GPU 上的完整 X25519 就绪。剩下两个入口函数：先讲匹配逻辑（6.5），再讲命中回传（6.6）。
