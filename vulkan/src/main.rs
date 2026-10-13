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

/// build.rs 用 glslangValidator 编译产物
const SHADER_SPV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/shader.spv"));
const SELFTEST_SPV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/selftest.spv"));

/// 每轮最多回收的命中数量（与 shader 中 slot < 64 对应）
const MAX_HITS: usize = 64;

/// push constant 布局，必须与 shader.comp 中的 `Push` 块一致
#[repr(C)]
#[derive(Clone, Copy, vulkano::buffer::BufferContents)]
struct Push {
    seed: u64,
    patlen: u32,
    pat: [u32; 11],
    mask: [u32; 11],
    pad: u32, // 对齐到 8 字节（104），与 shader 中 Push 块一致
}

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

struct Gpu {
    device: Arc<Device>,
    queue: Arc<Queue>,
    queue_family_index: u32,
}

fn init() -> Result<Gpu, Box<dyn std::error::Error>> {
    let library = VulkanLibrary::new()?;
    let instance = Instance::new(library, InstanceCreateInfo::default())?;

    let physical = instance
        .enumerate_physical_devices()?
        .filter(|p| p.supported_features().shader_int64)
        .max_by_key(|p| match p.properties().device_type {
            PhysicalDeviceType::DiscreteGpu => 3,
            PhysicalDeviceType::IntegratedGpu => 2,
            _ => 1,
        })
        .ok_or("没有支持 shaderInt64 的 Vulkan 设备")?;
    eprintln!("使用设备: {}", physical.properties().device_name);

    let queue_family_index = physical
        .queue_family_properties()
        .iter()
        .position(|q| q.queue_flags.contains(QueueFlags::COMPUTE))
        .ok_or("没有支持 compute 的队列族")? as u32;

    let (device, mut queues) = Device::new(
        physical,
        DeviceCreateInfo {
            queue_create_infos: vec![QueueCreateInfo {
                queue_family_index,
                ..Default::default()
            }],
            enabled_features: DeviceFeatures {
                shader_int64: true,
                ..DeviceFeatures::empty()
            },
            ..Default::default()
        },
    )?;

    Ok(Gpu {
        device,
        queue: queues.next().unwrap(),
        queue_family_index,
    })
}

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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut prefix_arg: Option<String> = None;
    let mut count: usize = 1;
    let mut round_log2: u32 = 22;
    let mut selftest: Option<usize> = None;

    let mut args = std::env::args().skip(1).peekable();
    while let Some(a) = args.next() {
        match a.as_str() {
            "--count" => count = args.next().ok_or("--count 需要参数")?.parse()?,
            "--round-log2" => round_log2 = args.next().ok_or("--round-log2 需要参数")?.parse()?,
            "--selftest" => {
                let n = args
                    .next_if(|s| s.parse::<usize>().is_ok())
                    .map(|s| s.parse().unwrap())
                    .unwrap_or(4096);
                selftest = Some(n);
            }
            "--help" | "-h" => {
                println!("用法: wg-vanity-vulkan <前缀> [--count N] [--round-log2 N] [--selftest [N]]");
                return Ok(());
            }
            s if s.starts_with('-') => return Err(format!("未知参数 {s}").into()),
            s => prefix_arg = Some(s.to_string()),
        }
    }

    let gpu = init()?;
    let allocator = Arc::new(StandardMemoryAllocator::new_default(gpu.device.clone()));
    // HOST_RANDOM_ACCESS：GPU 写、CPU 读的小 buffer
    let mem_filter = MemoryTypeFilter::PREFER_DEVICE | MemoryTypeFilter::HOST_RANDOM_ACCESS;

    if let Some(n) = selftest {
        // workgroup 大小是 128，向上取整避免越界写
        let n = n.next_multiple_of(128);
        let pipeline = make_pipeline(&gpu.device, &load_shader(&gpu.device, SELFTEST_SPV)?)?;
        let buf: Subbuffer<[u64]> = Buffer::new_slice(
            allocator.clone(),
            BufferCreateInfo {
                usage: BufferUsage::STORAGE_BUFFER,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: mem_filter,
                ..Default::default()
            },
            (n * 4) as u64,
        )?;
        let seed: u64 = 0xDEAD_BEEF_CAFE_F00D;
        let push = Push { seed, patlen: 0, pat: [0; 11], mask: [0; 11], pad: 0 };
        let groups = (n / 128) as u32;
        dispatch(&gpu, &pipeline, &buf, push, groups)?;

        let content = buf.read()?;
        for gid in 0..n {
            let mut pub_bytes = [0u8; 32];
            for i in 0..4 {
                pub_bytes[i * 8..i * 8 + 8].copy_from_slice(&content[gid * 4 + i].to_le_bytes());
            }
            let key = keygen::derive_key(seed, gid as u64);
            let expect = curve25519_dalek::constants::X25519_BASEPOINT
                .mul_clamped(key)
                .to_bytes();
            if pub_bytes != expect {
                return Err(format!(
                    "自检失败：gid={} GPU={} 参考={}",
                    gid,
                    STANDARD.encode(pub_bytes),
                    STANDARD.encode(expect)
                )
                .into());
            }
        }
        println!("自检通过：{n} 个密钥与 curve25519-dalek 一致");
        return Ok(());
    }

    let prefix = prefix_arg.ok_or("缺少前缀参数（--help 查看用法）")?;
    let pattern = prefix::from_prefix(&prefix)?;
    let estimate = 64f64.powi(prefix.len() as i32);
    println!("前缀 {prefix:?}：期望尝试次数 ≈ 2^{:.1}", estimate.log2());

    let pipeline = make_pipeline(&gpu.device, &load_shader(&gpu.device, SHADER_SPV)?)?;
    // 布局与 shader 的 Results 块一致：word0 = count，其后 64 个 slot × 4 个 u64
    let buf: Subbuffer<[u64]> = Buffer::new_slice(
        allocator.clone(),
        BufferCreateInfo {
            usage: BufferUsage::STORAGE_BUFFER,
            ..Default::default()
        },
        AllocationCreateInfo {
            memory_type_filter: mem_filter,
            ..Default::default()
        },
        (1 + MAX_HITS * 4) as u64,
    )?;

    let round_size = 1usize << round_log2;
    let groups = (round_size / 128) as u32;
    let seed0 = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos() as u64;

    let mut total: u64 = 0;
    let mut collected = 0usize;
    let t_start = Instant::now();
    for round in 0u64.. {
        // 清零命中计数
        buf.write()?[0] = 0;

        let push = Push {
            seed: seed0.wrapping_add(round << round_log2),
            patlen: pattern.len as u32,
            pat: pack_words(&pattern.pat),
            mask: pack_words(&pattern.mask),
            pad: 0,
        };
        dispatch(&gpu, &pipeline, &buf, push, groups)?;
        total += round_size as u64;

        let found = (buf.read()?[0] & 0xFFFF_FFFF) as usize;
        if found > 0 {
            let n = found.min(MAX_HITS);
            let content = buf.read()?;
            for slot in 0..n {
                let mut key = [0u8; 32];
                for i in 0..4 {
                    key[i * 8..i * 8 + 8]
                        .copy_from_slice(&content[1 + slot * 4 + i].to_le_bytes());
                }
                let pub_bytes = x25519::x25519_base(&key);
                debug_assert!(pattern.matches(&pub_bytes));
                println!("私钥: {}", STANDARD.encode(key));
                println!("公钥: {}", STANDARD.encode(pub_bytes));
                println!();
            }
            collected += n;
            if collected >= count {
                break;
            }
        }

        if round % 10 == 9 {
            let secs = t_start.elapsed().as_secs_f64();
            eprintln!(
                "已尝试 {:.2}e9 次，速率 {:.1} M/s",
                total as f64 / 1e9,
                total as f64 / secs / 1e6
            );
        }
    }

    let secs = t_start.elapsed().as_secs_f64();
    eprintln!(
        "完成：共尝试 {} 次，耗时 {:.1}s，平均 {:.1} M/s",
        total,
        secs,
        total as f64 / secs / 1e6
    );
    Ok(())
}
