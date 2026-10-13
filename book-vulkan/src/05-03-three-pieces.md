# 5.3 管线、descriptor set、push constant 三件套

这节用本项目的参数传输当例子，讲清 GPU 程序「怎么拿到数据」的三个机制。第 6、7 章会反复用到这三个词。

## 场景

shader 里的 `main()` 没有参数、没有返回值。每轮搜索，CPU 要告诉 GPU：种子 `seed`、前缀模式 `pat`/`mask`；GPU 要回传：命中了几个、命中的私钥。这些数据怎么过境？

## ComputePipeline（管线）

管线 = 「要跑哪个 shader」加上一堆固定配置（入口函数、workgroup 大小、descriptor 布局……）的打包对象。Vulkan 要求事先把管线整个创建好，执行时一次性绑定——驱动据此提前完成所有编译和状态检查，执行时零意外。

类比：管线是「模具」，dispatch 是「按模具压 N 次」。我们的模具里装着编译好的搜索 shader。

## DescriptorSet（描述符集）：传大块数据

**descriptor** 是「指向一块资源的句柄」——对我们就一种：`buffer`（SSBO，shader storage buffer object，一块 GPU 可读写、CPU 也可访问的内存）。

descriptor set 是一组 descriptor 的打包，shader 里用编号引用：

```glsl
layout(set = 0, binding = 0) buffer Results { ... };
```

`set = 0, binding = 0` 就是地址：第 0 组第 0 个槽。CPU 侧创建 buffer、把它的 descriptor 写进 set、绑定到管线——shader 里的 `Results` 就和那块内存连上了。

💡 **补给：为什么要绕这一层。** GPU 和 CPU 不共享指针。buffer 是驱动分配的显存/内存对象，descriptor 是 shader 可见的「门票」：它同时声明了这块内存的类型、大小、访问方式，驱动在绑定瞬间完成所有地址翻译和权限检查。

适合传：**大块、每轮不变或 GPU 要回写的数据**。我们的命中结果 buffer 就是 descriptor 传的。

## push constant：传小参数

每轮都要换一个 `seed`，为它建 buffer 太重。**push constant** 是嵌在命令缓冲里的一小块数据（Vulkan 保证至少 128 字节），随 dispatch 命令一起发出去，shader 里当成一个 uniform 块直接读：

```glsl
layout(push_constant) uniform Push {
    uint64_t seed;
    ...
} pc;
```

我们的 `Push` 块正好 104 字节（seed 8 + patlen 4 + pat 44 + mask 44 + pad 4），在 128 字节保底之内。适合传：**小、每轮都变、只读**的数据——种子和前缀模式完美符合。

## 选择口诀

| 数据 | 大小 | 每轮变？ | GPU 写？ | 机制 |
|---|---|---|---|---|
| seed / 前缀 | 104 B | 变 | 否 | push constant |
| 命中结果 | ~2 KB | — | 是 | descriptor → SSBO |

「小参数走 push constant，大 buffer 走 descriptor」——第 6、7 章你会看到这条口诀落在每一行代码上。
