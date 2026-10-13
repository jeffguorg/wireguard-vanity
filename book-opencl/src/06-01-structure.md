# 6.1 kernel 总体结构

这节创建 `kernel.cl`，看清它的整体地图，并敲下文件头。从本节到 6.7，所有代码逐段拼起来就是完整的最终 kernel。

## 设计：一个 work-item = 一个候选密钥

回到 3.7 节的模型，落到具体流程，每个 work-item 做四件事：

```text
gid ──①derive_key──► 候选私钥（4×u64，已 clamp）
    ──②wg_pubkey───► X25519 梯子 + 求逆 → 公钥 32 字节
    ──③前缀比对────► (pub[i] & mask[i]) != pat[i] 就退出
    ──④命中回传────► atomic_inc 拿一个 slot，写入 keys
```

## 文件地图

`kernel.cl` 共 330 余行，分布如下（右边是讲解它的节）：

| 区段 | 内容 | 小节 |
|---|---|---|
| 文件头 | 类型别名、fe 结构体、掩码常量 | 6.1（本节） |
| `sm64_next` | splitmix64 随机数 | 6.2 |
| `fe_carry` … `fe_mul121665` | 域运算（对应 fe.rs） | 6.3 |
| `fe_invert` / `fe_tobytes` | 求逆 / 打包 | 6.4 |
| `cswap` / `wg_pubkey` | 梯子主体（对应 x25519.rs） | 6.4 |
| `derive_key` | 私钥派生（对应 keygen.rs） | 6.2 |
| `__kernel vanity` | 搜索入口 | 6.5、6.6 |
| `__kernel selftest_keys` | 自检入口 | 6.7 |

## 文件头

在项目根目录（`Cargo.toml` 旁边，**不在 src/ 里**）新建 `kernel.cl`，敲入：

```c
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
```

逐行说明：

- OpenCL C 没有 `u64` 这个名字，内置类型叫 `ulong`（64 位无符号）。三行 typedef 让 kernel 代码长得和 Rust 版几乎一样，对照翻译时不容易错；
- `fe` 结构体对应 Rust 的 `Fe(pub [u64; 10])`；
- `M26`/`M25` 用宏写出十六进制值：0x3ffffff = 2²⁶−1，0x1ffffff = 2²⁵−1，和 Rust 版的 `(1 << 26) - 1` 相同。

> 💡 补给：OpenCL C 是 C99 的超集：有 `__kernel`/`__global` 等修饰符和 `get_global_id` 等内建函数，没有 C++ 的模板/引用。数值溢出规则和 Rust 的 `wrapping_*` 一样：无符号数自然回绕，所以移植 splitmix64 时反而更简单。

## 小结

骨架就位。下一节写「gid → 私钥」的派生逻辑，它是整个搜索的源头。
