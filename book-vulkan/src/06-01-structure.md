# 6.1 shader 总体结构与 #ifdef SELFTEST 双版本

这章把第 4 章验证过的数学逐行翻译成 GLSL，写出 `shader.comp`。这节搭骨架：文件头、workgroup 大小、双版本 buffer 声明，并给出整份文件的地图。

## 文件头

在项目根目录（与 `Cargo.toml` 同级）创建 `shader.comp`：

```glsl
// WireGuard 公钥前缀搜索 compute shader（GLSL → SPIR-V）。
//
// 与 OpenCL 版 kernel.cl、宿主端 Rust 参考实现（src/fe.rs 等）一一对应。
// 编译两次：默认生成 vanity 入口；加 -DSELFTEST 生成自检入口。

#version 450
#extension GL_EXT_shader_explicit_arithmetic_types_int64 : require

layout(local_size_x = 128) in;
```

- `#version 450`：GLSL 版本，Vulkan 要求的最低版本；
- `#extension ... int64 : require`：5.2 节说过，开 64 位整数；
- `local_size_x = 128`：一个 workgroup（线程组）128 个线程。dispatch 时给的组数 × 128 = 本轮密钥总数。128 是实验后选的值，8.2 节有对比。

## 一份源码，两个入口

同一份文件编译两次（`-DSELFTEST` 是 glslangValidator 的预处理宏开关）：

- **vanity 版**（默认）：每个线程算一个公钥、比对前缀，命中就把私钥写进 `Results` buffer；
- **selftest 版**：不比前缀，把完整公钥写进 `Pubs` buffer，供 CPU 与参考实现逐一比对——这是我们调试 GPU 代码的「眼睛」。

buffer 声明用预处理分开：

```glsl
#ifdef SELFTEST
layout(set = 0, binding = 0) writeonly buffer Pubs {
    uint64_t pubs[];
};
#else
layout(set = 0, binding = 0) buffer Results {
    uint count;     // 命中计数（atomicAdd）
    uint64_t keys[]; // 每个命中写 4 个 u64（私钥）
};
#endif
```

💡 **补给：#ifdef。** 和 C 的预处理器一样：编译时给了 `-DSELFTEST` 就保留第一支，否则保留第二支。两个版本共享全部域运算代码，只有入口行为不同——保证「自检通过的代码」和「实际搜索的代码」是同一套数学。

`set = 0, binding = 0` 就是 5.3 节讲的 descriptor 地址。两个版本都只占这一个槽位。

## 整份文件地图

后面六节按讲解顺序覆盖所有函数；**在文件中的最终位置**以此表为准（GLSL 要求函数先声明后调用，这个顺序满足）：

```text
文件头（本节）→ push constant 块（6.2）→ buffer 声明（本节）
→ fe 结构体与常量（6.4）→ sm64_next（6.3）→ fe_carry/fe_add/fe_sub/fe_mul（6.4）
→ fe_sq/fe_mul121665/fe_invert/cswap/wg_pubkey（6.5）→ fe_towords（6.6）
→ derive_key（6.3 讲，放这里）→ main（6.6/6.7）
```

讲解顺序和文件顺序有两处不一致（derive_key 提前讲、fe_towords 推后讲），敲代码时按本表的位置放即可，到时各节会再提醒。

下一节：push constant 块——CPU 和 GPU 之间的 104 字节约定。
