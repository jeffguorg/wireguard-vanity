# 7.4 descriptor set 与 buffer

这节写 `dispatch()`——一轮 GPU 计算的完整提交过程：建 descriptor set、录命令、提交、fence 等待。buffer 的创建在 main 里（7.5/7.6 节），但含义在这里一起讲。

## dispatch()

接在 `init()` 之后：

```rust
/// 在 GPU 上跑一轮：dispatch groups 个 workgroup，等待完成。
fn dispatch(
    gpu: &Gpu,
    pipeline: &Arc<ComputePipeline>,
    buf: &Subbuffer<[u64]>,
    push: Push,
    groups: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    let ds_allocator = Arc::new(StandardDescriptorSetAllocator::new(gpu.device.clone(), Default::default()));
    let set = DescriptorSet::new(
        ds_allocator,
        pipeline.layout().set_layouts()[0].clone(),
        [WriteDescriptorSet::buffer(0, buf.clone())],
        [],
    )?;

    let cb_allocator =
        Arc::new(StandardCommandBufferAllocator::new(gpu.device.clone(), Default::default()));
    let mut builder = AutoCommandBufferBuilder::primary(
        cb_allocator,
        gpu.queue_family_index,
        CommandBufferUsage::OneTimeSubmit,
    )?;
    builder
        .bind_pipeline_compute(pipeline.clone())?
        .bind_descriptor_sets(
            PipelineBindPoint::Compute,
            pipeline.layout().clone(),
            0,
            set,
        )?
        .push_constants(pipeline.layout().clone(), 0, push)?;
    unsafe { builder.dispatch([groups, 1, 1]) }?;

    let future = sync::now(gpu.device.clone())
        .then_execute(gpu.queue.clone(), builder.build()?)?
        .then_signal_fence_and_flush()?;
    future.wait(None)?;
    Ok(())
}
```

## 逐段对照 5.1/5.3 节

1. **descriptor set**：`WriteDescriptorSet::buffer(0, buf)` 把 buffer 写到 0 号 binding——shader 里 `layout(set=0, binding=0)` 的那个槽。`set_layouts()[0]` 是管线的第 0 组布局（由反射生成，7.3 节）。
2. **录命令**：`AutoCommandBufferBuilder` 链式录四条命令：绑管线 → 绑 descriptor set → 推 push constant（`push` 整个结构体按值塞进去，104 字节）→ dispatch。
3. **dispatch 又是 unsafe**：vulkano 无法静态检查「线程总数 × 每组线程数是否越界访问 buffer」这类运行时属性，需要调用方担保。我们的担保：groups × 128 = 本轮密钥数，结果 buffer 足够大（MAX_HITS 闸门 + buffer 长度在 main 里配好）。
4. **提交与等待**：`sync::now` 造一个「当前时间线」起点 → `then_execute` 把命令缓冲挂上队列 → `then_signal_fence_and_flush` 提交并在末尾挂 fence → `future.wait(None)` 阻塞到 GPU 干完。

💡 **补给：fence 与异步。** GPU 是异步执行的：`then_execute` 之后 CPU 立刻可以继续跑。我们这个程序每轮都要等结果再决定下一步，所以直接 `wait`。想压榨性能可以「双缓冲」——提交第 n+1 轮后再等第 n 轮，见 8.2 练习。

## Subbuffer<[u64]> 是什么

参数类型 `&Subbuffer<[u64]>`：整个 buffer 被解释成 u64 数组，CPU 侧 `buf.read()` / `buf.write()` 拿到的是 `&[u64]` 的守卫。6.7 节的布局约定（content[0] 低 32 位是 count，其后是 keys）就是按这个视图定的。

💡 **补给：HOST_RANDOM_ACCESS。** main 里创建 buffer 时用的内存过滤条件，先记住含义：「优先显存，但 CPU 必须能直接随机读写」。GPU 写结果、CPU 读结果的小 buffer 正需要这种内存；AMD 平台的 Resizable BAR 让「显存 + CPU 可访问」两全。下一节看到完整创建代码。
