# 6.2 push constant 块布局

这节定义 shader 的参数块，并解释一个容易踩的坑：为什么末尾要塞一个 `pad`——它涉及 Rust 和 GLSL 两边内存布局的对齐约定。

## shader 侧

接在 6.1 的文件头之后（`layout(local_size_x...) in;` 与 buffer 声明之间）：

```glsl
// push constant：每轮不变的参数（种子、前缀模式）
layout(push_constant) uniform Push {
    uint64_t seed;
    uint patlen;
    uint pat[11];  // 44 字节，按小端打包进 uint
    uint mask[11];
    uint pad;      // 让块大小对齐到 8 字节（104），与 Rust 端结构体一致
} pc;
```

字段含义：`seed` 是本轮的种子；`patlen` 是要比对的字节数；`pat`/`mask` 是前缀翻译出的「字节模式 + 掩码」（3.1 节讲过由来），44 是按字符数取的宽松上限：43 个字符 × 6 bit = 258 bit，最多产生 33 字节；取 44 既保险，又正好是 11 个 uint。

💡 **补给：44 字节塞进 uint 数组。** push constant 的默认布局规则（std430 风格的 scalar 对齐）按 4 字节为颗粒。我们把 pat 的 44 个字节按小端序每 4 个打包进一个 `uint`，共 11 个：第 i 字节住在 `pat[i/4]` 的第 (i%4)×8 个 bit。6.6 节 shader 端抠字节、7.3 节 Rust 端打包，用的是同一套小端约定。

## 为什么要 pad 到 104

算一下：seed 8 + patlen 4 + pat 44 + mask 44 = **100 字节**。问题：

- GLSL 侧：`uint64_t seed` 要求 8 字节对齐，排在偏移 0 没问题；整个块 100 字节，语法上合法；
- Rust 侧（7.2 节）：对应的结构体是 `#[repr(C)] struct Push { seed: u64, patlen: u32, pat: [u32; 11], mask: [u32; 11], pad: u32 }`。Rust 结构体的大小必须是最大对齐（8 字节）的整数倍——没有 `pad` 的话 Rust 会算出 104（自动尾填充），而我们**显式**写出 `pad` 让两端逐字段对齐，谁也别偷偷猜。

于是 GPU 读 `pc.pat[10]` 的高字节和 Rust 写 `pat[10]` 的高字节，指的一定是同一位。push constant 是纯内存拷贝，两端布局必须**逐字节一致**，这类 bug 不报错、只给你错误结果，是最阴险的一类。

## CPU 侧：prefix.rs（生成 pat/mask 的模块）

pat/mask 是 CPU 从命令行前缀算出来的，趁讲布局把这个模块写完。创建 `src/prefix.rs`（它的测试模块在 7.7 节补上）：

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
    if prefix.is_empty() || prefix.len() > 43 {
        return Err(format!(
            "前缀长度需在 1..=43 之间（公钥 base64 共 44 字符，末位固定为 '='），当前 {}",
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
```

逻辑就是 3.1 节的「6n bit 拼字节流」：每个 base64 字符贡献 6 bit（高位在前），攒够 8 bit 吐一个字节、掩码全 1；结尾不足 8 bit 的尾巴左对齐，掩码只留高几位。

💡 **补给：Result 与 ?。** `Result<Pattern, String>` 表示「要么成功给你 Pattern，要么失败给你错误字符串」。`?` 运算符：值为 `Ok(x)` 就取出 x 继续，为 `Err(e)` 就当场把错误返回给调用者。`ok_or_else` 把 `Option` 变成 `Result`，配合 `?` 一行完成「非法字符就报错退出」。

这个文件现在还没被 `mod` 声明，放着即可，第 7 章 main.rs 会接它。
