# 5.2 kernel、work-item、global id

这节讲 GPU 侧的执行模型：kernel 是什么、一次发射怎么变成几百万次执行、每个执行怎么知道自己该算哪个候选。

## kernel = GPU 上的函数

kernel 是用 OpenCL C 写的函数，用 `__kernel` 标记，由宿主程序编译（在运行时用驱动自带的编译器！）后发射到 GPU。我们的 kernel 长这样（第 6 章逐行写）：

```c
__kernel void vanity(u64 seed,
                     __constant const u8 *pat,
                     __constant const u8 *mask,
                     u32 patlen,
                     __global u32 *found,
                     __global u64 *keys) {
    u64 gid = (u64)get_global_id(0);
    ...
}
```

## 一次发射 = N 个 work-item

宿主发射 kernel 时指定一个 **global work size**，比如 2²² = 4 194 304。GPU 就会创建这么多**个 work-item**，每个都独立执行一遍 kernel 函数——就像把同一个函数并发调用 400 万次。

work-item 之间唯一的区别是编号：`get_global_id(0)` 返回自己的编号（0 到 N−1）。我们的设计里，**gid 就是候选密钥的编号**：gid 派生出第 gid 个私钥。这样一次发射就检查了 400 万个候选。

```text
vanity kernel × 2^22 个 work-item
   gid=0  → 候选私钥 0  → 不匹配，悄悄退出
   gid=1  → 候选私钥 1  → 不匹配，悄悄退出
   ...
   gid=123456 → 命中！写入结果区
   ...
```

## work-group 与 local size（知道即可）

硬件上 work-item 按 **work-group** 分组执行（AMD 上一组最多 256 个，组内可共享局部内存）。发射时可以不指定 local size，让驱动自己选——我们的代码就是这么做的，候选之间不需要任何协作，分组方式不影响正确性。

> 💡 补给：OpenCL 里 `get_global_id(0)` 的 `0` 是「第 0 维」。kernel 可以按 1/2/3 维网格发射（图像处理常用二维），我们只需要一维。

## 小结

- kernel = 所有 work-item 共用的那段代码；
- global work size = 一次发射的候选数量；
- gid = 每个 work-item 的编号，我们用它当候选序号。

下一节讲数据怎么在 CPU 和 GPU 之间来回搬。
