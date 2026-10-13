# 6.7 命中回传：atomicAdd 与结果 buffer 布局

这节写 `main()` 的完整结构：每个线程干什么、命中了怎么写回、为什么需要原子操作。写完它，`shader.comp` 就完整了。

## main 全景

```glsl
// ---------------- 入口 ----------------

void main() {
    uint64_t gid = uint64_t(gl_GlobalInvocationID.x);
    uint64_t w[4];
    derive_key(pc.seed, gid, w);

    uint pw[8];
    wg_pubkey(w, pw);
```

`gl_GlobalInvocationID.x` 是内置变量：本线程的全局编号（0 到 线程总数−1）。每个线程拿它当 `gid`，派生私钥、算公钥——这就是 3.7 节「一个线程在干什么」的代码形态。

## selftest 分支：写回完整公钥

```glsl
#ifdef SELFTEST
    // 写回完整公钥，宿主端与参考实现比对
    uint gi = uint(gid) * 4u;
    pubs[gi + 0u] = uint64_t(pw[0]) | (uint64_t(pw[1]) << 32);
    pubs[gi + 1u] = uint64_t(pw[2]) | (uint64_t(pw[3]) << 32);
    pubs[gi + 2u] = uint64_t(pw[4]) | (uint64_t(pw[5]) << 32);
    pubs[gi + 3u] = uint64_t(pw[6]) | (uint64_t(pw[7]) << 32);
```

8 个 uint 两两拼成 4 个 u64 写进 `pubs`（低位字在前，仍是小端约定）。每个线程写自己的 4 个槽，互不重叠，不需要同步。

## vanity 分支：匹配 + 原子计数回写

#else 分支先跑 6.6 节的匹配循环，通过后：

```glsl
#else
    // （6.6 节的匹配循环在这里，不匹配就 return）
    uint slot = atomicAdd(count, 1u);
    if (slot < 64u) {
        keys[slot * 4u + 0u] = w[0];
        keys[slot * 4u + 1u] = w[1];
        keys[slot * 4u + 2u] = w[2];
        keys[slot * 4u + 3u] = w[3];
    }
#endif
}
```

💡 **补给：atomicAdd。** 一轮里可能多个线程同时命中，它们要争抢不同的槽位。`atomicAdd(count, 1u)` 把 `count` 加 1 并**返回加之前的值**——这个「读-改-写」由硬件保证不可分割，两个线程不会拿到同一个 slot。这是 GPU 编程里少数必需的同步原语之一。

为什么有 `slot < 64` 的闸门：buffer 是 CPU 侧定长分配的（64 个槽，7.5 节），一轮命中超过 64 个时多余的只计数、不写入，防止越界。短前缀（如 1 字符）一轮能命中几万个，反正我们只要 `--count` 个就够。

## 结果 buffer 的布局约定

```text
u64 视图下（CPU 侧按 [u64] 读）：
  [0]   低 32 位 = count（命中计数）
  [1 + slot*4 .. +3]  第 slot 个命中私钥的 4 个 u64
```

⚠️ **对齐坑**：shader 里 `Results` 块的第一个成员 `count` 是 4 字节的 `uint`，紧跟其后的 `keys[]` 是 8 字节对齐的 `uint64_t`——std430 布局会把 `keys` 放到偏移 8（count 占低 4 字节，浪费 4 字节）。而 CPU 侧把整个 buffer 当 `[u64]` 读：`content[0]` 的低 32 位是 count，`content[1]` 起才是 keys。所以 7.5 节读计数时写的是 `buf.read()?[0] & 0xFFFF_FFFF`——只取低 32 位。两边都对齐到 8 字节视图，是这个约定成立的原因。

## 编译验证

```console
$ glslangValidator -V --target-env vulkan1.1 shader.comp -o /tmp/shader.spv
shader.comp
$ glslangValidator -V --target-env vulkan1.1 -DSELFTEST shader.comp -o /tmp/selftest.spv
shader.comp
```

两条都不报错，shader 章节就完成了（第 7 章的 build.rs 会自动跑这两条命令）。如果报错，glslangValidator 会给出文件名和行号，回去对照章节代码块即可。
