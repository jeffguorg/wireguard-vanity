# 5.2 compute shader 与 SPIR-V

这节讲 GPU 程序的存在形式：为什么 Vulkan 的 shader 要先离线编译成 SPIR-V，并动手编一次看看。

## compute shader

shader（着色器）原本是图形流水线上的小程序；**compute shader** 是脱离了图形管线的纯计算 shader：不传顶点、不输出像素，就是「N 个线程各跑一遍 `main()`」。我们的搜索逻辑整个在一个 compute shader 里。

## SPIR-V：Vulkan 的 shader 中间码

Vulkan 驱动**不接受 GLSL 源码**。无论你用什么语言写 shader，都必须先编译成 **SPIR-V**——一种二进制中间表示（IR），再由驱动在加载时编译成具体 GPU 的机器码。

💡 **补给：为什么多这一层。** GLSL 源码编译慢、各家驱动的前端还可能有细微差异；SPIR-V 把「语言编译」和「机器码生成」分开：glslangValidator 离线做前者（慢但只做一次），驱动只做后者（快、统一）。这也是 Vulkan 启动比 OpenGL 可预测的原因之一。类比：GLSL 之于 SPIR-V，如 Rust 源码之于 LLVM IR。

## 手动编一次

虽然我们还没有写项目的 shader，先用个最小例子体验（在任意目录）：

```console
$ cat > mini.comp << 'EOF'
#version 450
layout(local_size_x = 64) in;
layout(set = 0, binding = 0) buffer Data { uint x[]; };
void main() { x[gl_GlobalInvocationID.x] *= 2u; }
EOF
$ glslangValidator -V --target-env vulkan1.1 mini.comp -o mini.spv
mini.comp
$ ls -l mini.spv
-rw-r--r-- 1 user user ... mini.spv
```

没报错、吐出 `.spv` 文件就成功了。`-V` 表示「生成 SPIR-V」，`--target-env vulkan1.1` 指定目标 Vulkan 版本（我们的项目也用这两个参数，第 7 章 `build.rs` 里会原样出现）。

看一眼二进制里嵌的字符串：

```console
$ strings mini.spv | head
```

能看到 `Data`、`main` 等符号——驱动加载时靠这些反射信息把 shader 和 buffer 对上号。

## int64 扩展

我们的 shader 第一行后面跟着一句：

```glsl
#extension GL_EXT_shader_explicit_arithmetic_types_int64 : require
```

GLSL 450 默认没有 64 位整数类型，需要显式开启这个扩展才能用 `uint64_t`；同时设备侧也要有 `shaderInt64` 特性（2.3 节验证过）。两头都满足，`uint64_t` 才可用。

## 项目里的编译时机

第 7 章会把「编译 shader」固化进 `build.rs`：每次 `cargo build` 自动调 glslangValidator 把 `shader.comp` 编成 SPIR-V，Rust 代码用 `include_bytes!` 把字节直接嵌进可执行文件——发布时不用附带额外文件。
