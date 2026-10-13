# 7.1 build.rs：构建期编译 shader

这章写宿主程序 `src/main.rs`。这节先做自动化：让 `cargo build` 自动编译 shader，并把 main.rs 的文件头写好。

## build.rs

在项目根目录（与 `Cargo.toml` 同级）创建 `build.rs`：

```rust
//! 构建脚本：用 glslangValidator 把 shader.comp 编译成 SPIR-V。
//! 同一份源码编译两次：正常版（vanity）与自检版（-DSELFTEST）。

use std::process::Command;

fn main() {
    let out_dir = std::env::var("OUT_DIR").unwrap();
    for (name, define) in [("shader", ""), ("selftest", "SELFTEST")] {
        let dst = format!("{out_dir}/{name}.spv");
        let mut cmd = Command::new("glslangValidator");
        cmd.args(["-V", "--target-env", "vulkan1.1", "shader.comp", "-o", &dst]);
        if !define.is_empty() {
            cmd.arg(format!("-D{define}"));
        }
        let status = cmd
            .status()
            .expect("无法运行 glslangValidator（请先安装 glslang）");
        assert!(status.success(), "glslangValidator 编译 {name} 失败");
    }
    println!("cargo:rerun-if-changed=shader.comp");
}
```

💡 **补给：build.rs 与 OUT_DIR。** cargo 在编译 crate 之前先编译并运行 `build.rs`。它通过环境变量通信：`OUT_DIR` 是 cargo 给的输出目录（`target/.../build/xxx/out`），最后一行 `cargo:rerun-if-changed=` 告诉 cargo「只有 shader.comp 变了才重跑我」。命令行参数和 6.7 节手动编译时一模一样，只是包进了循环。

## main.rs：整份替换占位版

第 4 章的占位 main.rs 现在整份替换。先写文件头（mod 声明 + import）：

```rust
//! WireGuard 公钥前缀搜索（Vulkan 版）宿主程序。
//!
//! 用法与 OpenCL 版相同：
//!   wg-vanity-vulkan <前缀> [--count N] [--round-log2 N] [--selftest [N]]

mod fe;
mod keygen;
mod prefix;
mod x25519;

use std::sync::Arc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use base64::{engine::general_purpose::STANDARD, Engine as _};
use vulkano::buffer::{Buffer, BufferCreateInfo, BufferUsage, Subbuffer};
use vulkano::command_buffer::allocator::StandardCommandBufferAllocator;
use vulkano::command_buffer::{AutoCommandBufferBuilder, CommandBufferUsage};
use vulkano::descriptor_set::allocator::StandardDescriptorSetAllocator;
use vulkano::descriptor_set::{DescriptorSet, WriteDescriptorSet};
use vulkano::device::physical::PhysicalDeviceType;
use vulkano::device::{Device, DeviceCreateInfo, DeviceFeatures, Queue, QueueCreateInfo, QueueFlags};
use vulkano::instance::{Instance, InstanceCreateInfo};
use vulkano::memory::allocator::{AllocationCreateInfo, MemoryTypeFilter, StandardMemoryAllocator};
use vulkano::pipeline::compute::ComputePipelineCreateInfo;
use vulkano::pipeline::layout::PipelineDescriptorSetLayoutCreateInfo;
use vulkano::pipeline::{
    ComputePipeline, Pipeline, PipelineBindPoint, PipelineLayout, PipelineShaderStageCreateInfo,
};
use vulkano::shader::{ShaderModule, ShaderModuleCreateInfo};
use vulkano::sync::{self, GpuFuture};
use vulkano::VulkanLibrary;
```

注意 `mod prefix;` 出现了——6.2 节写的 `src/prefix.rs` 从这里开始正式接入编译。import 清单看着长，其实就四类：buffer、command buffer、descriptor、pipeline/device/instance，7.2~7.4 会逐个用到。

> 📝 替换占位时，**暂时保留文件末尾的 `fn main() {}`**，否则 bin crate 没有入口编译不过。真正的 main 在 7.5 节写。

后面三节按文件顺序补全：常量与 Push 结构体 + 初始化（7.2）、加载 shader 与管线（7.3）、dispatch（7.4）。
