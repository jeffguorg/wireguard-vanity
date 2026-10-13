# 7.3 加载 SPIR-V 与创建 compute pipeline

这节写三个小函数：`pack_words`（前缀字节打包）、`load_shader`（SPIR-V → ShaderModule）、`make_pipeline`（ShaderModule → ComputePipeline）。按 7.1 的地图，它们放在 `Push` 结构体之后、`Gpu`/`init()` 之前。

## pack_words 与 load_shader

```rust
fn pack_words(bytes: &[u8; 44]) -> [u32; 11] {
    let mut w = [0u32; 11];
    for (i, &b) in bytes.iter().enumerate() {
        w[i / 4] |= (b as u32) << ((i % 4) * 8);
    }
    w
}

fn load_shader(device: &Arc<Device>, spv: &[u8]) -> Result<Arc<ShaderModule>, Box<dyn std::error::Error>> {
    let words: Vec<u32> = spv
        .chunks_exact(4)
        .map(|c| u32::from_le_bytes(c.try_into().unwrap()))
        .collect();
    Ok(unsafe { ShaderModule::new(device.clone(), ShaderModuleCreateInfo::new(&words)) }?)
}
```

- `pack_words` 就是 6.2 节的约定：44 字节按小端每 4 个装进一个 u32。shader 里 `(pat[i>>2] >> ((i&3)*8)) & 0xFF` 是它的逆操作。
- `load_shader`：SPIR-V 是 u32 数组，先把 `&[u8]` 按 4 字节一组转 `Vec<u32>`（`from_le_bytes`，SPIR-V 规范就是小端），再交给 vulkano。

💡 **补给：unsafe。** `ShaderModule::new` 是 unsafe，因为 vulkano 无法验证这段字节是不是合法 SPIR-V——喂给它乱码可能导致驱动崩溃（内存安全意义上的 UB）。这里安全由两步保证：字节来自 glslangValidator 的编译产物（build.rs 里 assert 了编译成功），且经 `include_bytes!` 嵌入不可篡改。 unsafe 块的意思是「我向编译器保证：这里的人祸我检查过了」。

💡 **补给：迭代器与 map/collect。** `chunks_exact(4)` 把切片切成 4 字节一组的迭代器；`.map(|c| ...)` 对每组做变换；`.collect()` 收成 `Vec`。三段链等价于一个 for 循环，但零开销且更声明式。`|c| ...` 是闭包（4.7 节补给过）。

## make_pipeline

```rust
fn make_pipeline(
    device: &Arc<Device>,
    shader: &Arc<ShaderModule>,
) -> Result<Arc<ComputePipeline>, Box<dyn std::error::Error>> {
    let entry = shader.entry_point("main").ok_or("shader 缺少 main 入口")?;
    let stage = PipelineShaderStageCreateInfo::new(entry);
    let layout = PipelineLayout::new(
        device.clone(),
        PipelineDescriptorSetLayoutCreateInfo::from_stages([&stage])
            .into_pipeline_layout_create_info(device.clone())?,
    )?;
    Ok(ComputePipeline::new(
        device.clone(),
        None,
        ComputePipelineCreateInfo::stage_layout(stage, layout),
    )?)
}
```

三步，对应 5.3 节的「模具」：

1. **entry_point("main")**：按名字找 shader 的入口函数。这个名字在 SPIR-V 里（5.2 节 `strings mini.spv` 见过符号）。
2. **PipelineLayout**：描述「这个管线用到几个 descriptor set、各是什么布局、push constant 多大」。`from_stages([&stage])` 让 vulkano **从 shader 反射自动推导**——不用手填，shader 里 `set = 0, binding = 0` 和 push constant 块会被自动识别。
3. **ComputePipeline::new**：真正创建管线。中间的 `None` 是缓存句柄（pipeline cache，可加速重复创建，我们不用）。

之后 `dispatch`（7.4）每轮要做的只是：绑管线、绑 descriptor set、推 push constant、发 dispatch——所有「重」的编译检查在这一步已经完成了。
