# 5.3 buffer 与 host↔device 传输

这节讲数据怎么进显存、结果怎么回来。CPU（host）和 GPU（device）各有自己的内存，互相看不见，一切数据交换都要显式搬运。

## buffer = 显存里的数组

OpenCL 里的 `Buffer` 是一块 device 侧内存，创建时指定类型、长度、访问模式。我们项目里用到四个（7.2 节的真实代码）：

```rust
let mut pat_buf = unsafe { Buffer::<u8>::create(&context, CL_MEM_READ_ONLY, 44, ptr::null_mut()) }?;
let mut mask_buf = unsafe { Buffer::<u8>::create(&context, CL_MEM_READ_ONLY, 44, ptr::null_mut()) }?;
let mut found_buf = unsafe { Buffer::<cl_uint>::create(&context, CL_MEM_READ_WRITE, 1, ptr::null_mut()) }?;
let keys_buf =
    unsafe { Buffer::<cl_ulong>::create(&context, CL_MEM_READ_WRITE, MAX_HITS * 4, ptr::null_mut()) }?;
```

- `pat_buf` / `mask_buf`：前缀的模式和掩码（6.5 节讲），kernel 只读 → `CL_MEM_READ_ONLY`；
- `found_buf`：命中计数器，kernel 要写 → `CL_MEM_READ_WRITE`；
- `keys_buf`：命中的私钥，64 个 slot × 4 个 u64。

## 搬运靠命令队列

写（host → device）：

```rust
unsafe { queue.enqueue_write_buffer(&mut pat_buf, CL_BLOCKING, 0, &pattern.pat, &[]) }?;
```

读（device → host）：

```rust
unsafe { queue.enqueue_read_buffer(&found_buf, CL_BLOCKING, 0, &mut found, &[]) }?;
```

参数依次是：目标 buffer、`CL_BLOCKING`（阻塞：等搬完才返回）、偏移、数据切片、依赖事件（我们没有，空数组）。

> 💡 补给：`CL_BLOCKING` 让调用线程等到传输完成，简单粗暴；非阻塞版本会立即返回、之后查事件，适合重叠计算与传输。我们的流程里读 `found` 本来就是「等 GPU 算完这轮」的同步点，阻塞正合适。

## 带宽账

我们的 kernel 几乎不搬数据：每轮写 1 个计数器、读 1 个计数器，命中时才读 64×32 字节。一轮 2²² 个候选只搬几十字节——这就是 3.7 节说的「算多访少」，PCIe 带宽完全不是瓶颈。

## 小结

- buffer 分 read-only / read-write，创建时定死；
- 读写都通过队列 enqueue，`CL_BLOCKING` = 等它完；
- kernel 的参数就是 buffer 的句柄，下一节看怎么把它们绑到 kernel 上。
