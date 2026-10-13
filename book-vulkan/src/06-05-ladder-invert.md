# 6.5 梯子与求逆的移植

这节是 shader 的主体：fe_sq、fe_mul121665、fe_invert、cswap、wg_pubkey（梯子主循环）。全部直译自第 4 章。

## fe_sq 与 fe_mul121665

```glsl
fe fe_sq(fe f) {
    uint64_t f0=f.v[0], f1=f.v[1], f2=f.v[2], f3=f.v[3], f4=f.v[4];
    uint64_t f5=f.v[5], f6=f.v[6], f7=f.v[7], f8=f.v[8], f9=f.v[9];

    uint64_t f0_2 = 2*f0, f1_2 = 2*f1, f2_2 = 2*f2, f3_2 = 2*f3, f4_2 = 2*f4;
    uint64_t f5_2 = 2*f5, f6_2 = 2*f6, f7_2 = 2*f7;
    uint64_t f5_38 = 38*f5, f6_19 = 19*f6, f7_38 = 38*f7, f8_19 = 19*f8, f9_38 = 38*f9;

    fe h;
    h.v[0] = f0*f0 + f1_2*f9_38 + f2_2*f8_19 + f3_2*f7_38 + f4_2*f6_19 + f5*f5_38;
    h.v[1] = f0_2*f1 + f2*f9_38 + f3_2*f8_19 + f4*f7_38 + f5_2*f6_19;
    h.v[2] = f0_2*f2 + f1_2*f1 + f3_2*f9_38 + f4_2*f8_19 + f5_2*f7_38 + f6*f6_19;
    h.v[3] = f0_2*f3 + f1_2*f2 + f4*f9_38 + f5_2*f8_19 + f6*f7_38;
    h.v[4] = f0_2*f4 + f1_2*f3_2 + f2*f2 + f5_2*f9_38 + f6_2*f8_19 + f7*f7_38;
    h.v[5] = f0_2*f5 + f1_2*f4 + f2_2*f3 + f6*f9_38 + f7_2*f8_19;
    h.v[6] = f0_2*f6 + f1_2*f5_2 + f2_2*f4 + f3_2*f3 + f7_2*f9_38 + f8*f8_19;
    h.v[7] = f0_2*f7 + f1_2*f6 + f2_2*f5 + f3_2*f4 + f8*f9_38;
    h.v[8] = f0_2*f8 + f1_2*f7_2 + f2_2*f6 + f3_2*f5_2 + f4*f4 + f9*f9_38;
    h.v[9] = f0_2*f9 + f1_2*f8 + f2_2*f7 + f3_2*f6 + f4_2*f5;
    return fe_carry(h);
}

fe fe_mul121665(fe f) {
    fe h;
    for (int i = 0; i < 10; i++) h.v[i] = f.v[i] * 121665;
    return h;
}
```

## fe_invert

加法链直译，`for _ in 1..N` 变成 `for (int i = 1; i < N; i++)`：

```glsl
fe fe_invert(fe z) {
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

## cswap 与 wg_pubkey（梯子主循环）

cswap 里 Rust 的 `0u64.wrapping_sub(swap)` 变成 `0UL - swap`（自动回绕）。wg_pubkey 即 `x25519_base`：初始化四元组、255 轮梯子、收尾 cswap、乘逆元、打包。私钥以 `w[4]`（4 个 u64）传入，取位用 `w[t >> 6] >> (t & 63)`（按 64 位字索引，等价 Rust 版按字节索引）：

```glsl
// ---------------- X25519 ----------------
void cswap(uint64_t swap, inout fe a, inout fe b) {
    uint64_t mask = 0UL - swap;
    for (int i = 0; i < 10; i++) {
        uint64_t t = mask & (a.v[i] ^ b.v[i]);
        a.v[i] ^= t;
        b.v[i] ^= t;
    }
}

// 公钥 = X25519(w, 基点 9)，输出为 8 个 uint
void wg_pubkey(uint64_t w[4], out uint pw[8]) {
    fe x1; x1.v[0] = 9;
    for (int i = 1; i < 10; i++) x1.v[i] = 0;
    fe x2; x2.v[0] = 1;
    for (int i = 1; i < 10; i++) x2.v[i] = 0;
    fe z2;
    for (int i = 0; i < 10; i++) z2.v[i] = 0;
    fe x3 = x1;
    fe z3 = x2;

    uint64_t swap = 0UL;
    for (int t = 254; t >= 0; t--) {
        uint64_t bit = (w[t >> 6] >> (t & 63)) & 1UL;
        swap ^= bit;
        cswap(swap, x2, x3);
        cswap(swap, z2, z3);
        swap = bit;

        fe a  = fe_add(x2, z2);
        fe aa = fe_sq(a);
        fe b  = fe_sub(x2, z2);
        fe bb = fe_sq(b);
        fe e  = fe_sub(aa, bb);
        fe c2 = fe_add(x3, z3);
        fe d  = fe_sub(x3, z3);
        fe da = fe_mul(d, a);
        fe cb = fe_mul(c2, b);
        fe dac = fe_add(da, cb);
        fe dmc = fe_sub(da, cb);

        x3 = fe_sq(dac);
        z3 = fe_mul(x1, fe_sq(dmc));
        x2 = fe_mul(aa, bb);
        z2 = fe_mul(e, fe_add(aa, fe_carry(fe_mul121665(e))));
    }
    cswap(swap, x2, x3);
    cswap(swap, z2, z3);

    fe_towords(fe_mul(x2, fe_invert(z2)), pw);
}
```

两处小改名：Rust 的 `c` 改成 `c2`（GLSL 里避免和常见变量混淆的口味选择），`da_plus_cb`/`da_minus_cb` 缩成 `dac`/`dmc`。`fe_towords` 还没写——下一节就是它，加上前缀匹配。

输出不是 32 字节而是「8 个 uint」：GPU 上字节操作麻烦，我们把公钥打包成 8 个 32 位字，前缀匹配和回写都按字处理（6.6/6.7 节）。
