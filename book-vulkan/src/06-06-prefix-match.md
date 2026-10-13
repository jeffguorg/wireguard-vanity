# 6.6 前缀匹配：字节从 uint 字里抠出来

这节写 `fe_towords`（强归约 + 打包成 8 个 uint）和 main 里的前缀匹配循环——「前缀模式」与「公钥字」在这里相遇。

## fe_towords

对应 Rust 的 `to_bytes`（4.7 节），但终点不是字节数组而是 `uint pw[8]`。分三段：先 carry，再 q 链做条件减 p，最后逐字节拼装。按 6.1 的地图，它放在 `fe_invert` 之后、`cswap` 之前（`wg_pubkey` 要调用它，GLSL 要求先声明后调用）：

```glsl
// 强归约 + 打包成 8 个 uint（32 字节小端序）
void fe_towords(fe h, out uint pw[8]) {
    h = fe_carry(h);

    uint64_t q = (19 * h.v[9] + (1UL << 24)) >> 25;
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
    // 最终进位链：最高 limb 溢出的位代表 q·2^255，直接丢弃
    uint64_t c;
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

    uint64_t v0=h.v[0], v1=h.v[1], v2=h.v[2], v3=h.v[3], v4=h.v[4];
    uint64_t v5=h.v[5], v6=h.v[6], v7=h.v[7], v8=h.v[8], v9=h.v[9];
    uint b[32];
    b[0]  = uint(v0) & 0xFFu;
    b[1]  = uint(v0 >> 8) & 0xFFu;
    b[2]  = uint(v0 >> 16) & 0xFFu;
    b[3]  = uint((v0 >> 24) | (v1 << 2)) & 0xFFu;
    b[4]  = uint(v1 >> 6) & 0xFFu;
    b[5]  = uint(v1 >> 14) & 0xFFu;
    b[6]  = uint((v1 >> 22) | (v2 << 3)) & 0xFFu;
    b[7]  = uint(v2 >> 5) & 0xFFu;
    b[8]  = uint(v2 >> 13) & 0xFFu;
    b[9]  = uint((v2 >> 21) | (v3 << 5)) & 0xFFu;
    b[10] = uint(v3 >> 3) & 0xFFu;
    b[11] = uint(v3 >> 11) & 0xFFu;
    b[12] = uint((v3 >> 19) | (v4 << 6)) & 0xFFu;
    b[13] = uint(v4 >> 2) & 0xFFu;
    b[14] = uint(v4 >> 10) & 0xFFu;
    b[15] = uint(v4 >> 18) & 0xFFu;
    b[16] = uint(v5) & 0xFFu;
    b[17] = uint(v5 >> 8) & 0xFFu;
    b[18] = uint(v5 >> 16) & 0xFFu;
    b[19] = uint((v5 >> 24) | (v6 << 1)) & 0xFFu;
    b[20] = uint(v6 >> 7) & 0xFFu;
    b[21] = uint(v6 >> 15) & 0xFFu;
    b[22] = uint((v6 >> 23) | (v7 << 3)) & 0xFFu;
    b[23] = uint(v7 >> 5) & 0xFFu;
    b[24] = uint(v7 >> 13) & 0xFFu;
    b[25] = uint((v7 >> 21) | (v8 << 4)) & 0xFFu;
    b[26] = uint(v8 >> 4) & 0xFFu;
    b[27] = uint(v8 >> 12) & 0xFFu;
    b[28] = uint((v8 >> 20) | (v9 << 6)) & 0xFFu;
    b[29] = uint(v9 >> 2) & 0xFFu;
    b[30] = uint(v9 >> 10) & 0xFFu;
    b[31] = uint(v9 >> 18) & 0xFFu;

    for (int j = 0; j < 8; j++) {
        pw[j] = b[4*j] | (b[4*j+1] << 8) | (b[4*j+2] << 16) | (b[4*j+3] << 24);
    }
}
```

32 行 `b[...]` 和 Rust 版一一对应；`uint(x) & 0xFFu` 就是 Rust 的 `as u8`（截断取低 8 位）。最后 4 字节拼一个 uint，**小端**：`b[4j]` 是最低字节——和 6.2 节 push constant 里 pat/mask 的打包方向一致，两头才能对上。

## main 的前缀匹配段

main 的开头（派生私钥、算公钥）和结尾（回写）在 6.7 节；这里先看中间的匹配循环，它和 `Pattern::matches`（6.2 节）是同一逻辑的两种语言：

```glsl
    for (uint i = 0u; i < pc.patlen; i++) {
        uint sh = (i & 3u) * 8u;
        uint b  = (pw[i >> 2] >> sh) & 0xFFu;
        uint pb = (pc.pat[i >> 2] >> sh) & 0xFFu;
        uint mb = (pc.mask[i >> 2] >> sh) & 0xFFu;
        if ((b & mb) != pb) return;
    }
```

💡 **补给：从 uint 里抠第 i 个字节。** 第 i 字节住在第 `i >> 2`（= i/4）个字里，字节内偏移 `(i & 3) * 8`（= i%4 × 8）个 bit——右移这么多位再 `& 0xFF` 就取出来了。这就是 6.2 节「44 字节塞进 uint 数组」约定的解码侧。

不匹配就 `return`：这个线程直接收工，什么都不写。绝大多数线程在这一两步就退出了（GPU 按 32/64 线程一组锁步执行，提前 return 的线程只是陪跑）——这是本 shader 唯一的有意分支，代价可忽略。

下一节：命中之后怎么办——atomicAdd 和结果回传。
