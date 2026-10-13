# 6.3 splitmix64 与 clamp

这节写私钥派生：从 `(seed, gid)` 确定性生成一个合法私钥。它和 4.1 节 `keygen.rs` 是同一逻辑的两种语言，可以逐行对照。

## sm64_next

按 6.1 的文件地图，这个函数放在 fe 结构体之后（下一节会先把结构体补上；顺序以地图为准）：

```glsl
// ---------------- splitmix64 私钥派生 ----------------

uint64_t sm64_next(inout uint64_t s) {
    s += 0x9E3779B97F4A7C15UL;
    uint64_t z = s;
    z = (z ^ (z >> 30)) * 0xBF58476D1CE4E5B9UL;
    z = (z ^ (z >> 27)) * 0x94D049BB133111EBUL;
    return z ^ (z >> 31);
}
```

对照 Rust 版 `splitmix64`：

- Rust 的 `s: &mut u64` → GLSL 的 `inout uint64_t s`。
- Rust 的 `wrapping_add`/`wrapping_mul` → GLSL 里直接写 `+`、`*`，`uint64_t` 算术本来就回绕，语义自动一致。

💡 **补给：UL 后缀与 inout。** GLSL 里 `uint64_t` 字面量要加 `UL` 后缀（如 `0x9E3779B97F4A7C15UL`），否则按 32 位解析会被截断——这是移植时最容易漏的东西。`inout` 是参数传递模式：函数内对参数的修改会写回调用方的变量，相当于 Rust 的 `&mut`（GLSL 没有指针，靠参数模式表达）。

## derive_key

```glsl
void derive_key(uint64_t seed, uint64_t gid, out uint64_t w[4]) {
    uint64_t s = seed + gid * 0x9E3779B97F4A7C15UL;
    w[0] = sm64_next(s);
    w[1] = sm64_next(s);
    w[2] = sm64_next(s);
    w[3] = sm64_next(s);
    w[0] &= ~7UL;
    w[3] = (w[3] & 0x7FFFFFFFFFFFFFFFUL) | 0x4000000000000000UL;
}
```

- `out uint64_t w[4]`：数组参数 + `out` 模式 = Rust 里「返回 4 个 u64」的写法（GLSL 函数不能直接返回数组）。
- 最后两行就是 clamp 的 u64 形式：`w[0] &= ~7UL` 清最低 3 位（等价字节级 `k[0] &= 248`）；`w[3]` 清最高位、置次高位（等价 `k[31] &= 127; k[31] |= 64`）。和 Rust 版逐位相同。

## 为什么这套派生是安全的

- `gid * φ`：φ 是奇数，乘法模 2^64 可逆，所以不同 gid 得到不同的初始状态，一轮内私钥互不重复；
- splitmix64 是成熟的整数哈希，输出统计性质好；
- 宿主每轮换 seed（7.5 节：`seed0 + round × 2^round_log2`），保证轮与轮之间也不重复。

> 📝 组装提醒：`derive_key` 在最终文件里位于 `wg_pubkey`（6.5 节）之后、`main` 之前。现在可以先写到 sm64_next 后面，6.5 节敲完 wg_pubkey 后再挪下去（GLSL 只要求调用前先声明，挪动不影响编译）。

下一节：域运算的翻译规则。
