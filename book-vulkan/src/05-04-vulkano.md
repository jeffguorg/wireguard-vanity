# 5.4 vulkano crate 速览

这节认识我们将在 Rust 里使用的 Vulkan 绑定库 vulkano：它的两个「怪癖」（Arc 满天飞、版本变动快），以及本书钉死的版本。

## vulkano 是什么

vulkano 是 Vulkan API 的 Rust 安全封装：底层仍是 Vulkan 的那套对象（instance/device/buffer/pipeline……），但用 Rust 类型系统把大量「用错就崩」的规则变成编译错误。我们不直接调 Vulkan C API——那要手写成倍的样板和 unsafe。

## 为什么到处是 Arc

第 7 章你会看到满屏的 `Arc<Device>`、`Arc<ComputePipeline>`、`Arc<Queue>`。

💡 **补给：Arc。** `Arc<T>` 是原子引用计数的共享指针：clone 一个 Arc 只增加计数，不复制里面的东西；最后一个 Arc 销毁时里面的对象才被释放。Vulkan 对象之间互相引用（pipeline 引用 device、command buffer 引用 pipeline……），所有权是网状的不是树状的，Arc 正是为此而生。看到 `.clone()` 别慌：克隆的是句柄，不是 GPU 资源本体。

## 版本说明：以 Cargo.toml 为准

⚠️ **vulkano 的 API 变动非常快**，0.3x 时代几乎每个 minor 版本都有破坏性改动（类型改名、构造函数签名变化）。网上的教程、Stack Overflow 答案、甚至 AI 生成的代码，大概率是别的版本的，直接抄会编译不过。

本书所有代码针对 **vulkano 0.35**（`Cargo.toml` 里写死了 `vulkano = "0.35"`，cargo 会解析到 0.35.x）。如果你读这本书时 0.35 已经很旧，**不要**顺手升级——跟着 0.35 跑通一遍，再考虑迁移。

## 本书用到的组件地图

| vulkano 类型 | 角色 | 出现节 |
|---|---|---|
| `VulkanLibrary`, `Instance` | 加载驱动、建实例 | 7.2 |
| `PhysicalDeviceType`, `DeviceFeatures` | 挑显卡、开 shaderInt64 | 7.2 |
| `Device`, `Queue`, `QueueCreateInfo` | 逻辑设备与队列 | 7.2 |
| `ShaderModule` | 加载 SPIR-V | 7.3 |
| `ComputePipeline`, `PipelineLayout` | 管线 | 7.3 |
| `Buffer`, `Subbuffer`, `StandardMemoryAllocator` | 结果 buffer | 7.4–7.6 |
| `DescriptorSet`, `WriteDescriptorSet` | 绑定 buffer | 7.4 |
| `AutoCommandBufferBuilder` | 录命令 | 7.4/7.5 |
| `sync::now`, `GpuFuture` | 提交与 fence 等待 | 7.4/7.5 |

现在不必记，第 7 章每节会逐个展开。这个表是给你迷路时回来查的。

## 下一步

概念齐了。第 6 章写 GPU 上跑的 `shader.comp`——把第 4 章验证过的 Rust 数学逐行翻译成 GLSL。
