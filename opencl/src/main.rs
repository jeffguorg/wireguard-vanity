//! WireGuard 公钥前缀搜索（OpenCL 版）宿主程序。
//!
//! 用法:
//!   wg-vanity-opencl <前缀> [--count N] [--round-log2 N] [--selftest [N]]
//!
//! 例:
//!   wg-vanity-opencl Hello          # 找第一个公钥以 "Hello" 开头的密钥对
//!   wg-vanity-opencl Hi --count 3   # 找 3 个
//!   wg-vanity-opencl --selftest     # GPU 与 curve25519-dalek 对拍 4096 个密钥

mod fe;
mod keygen;
mod prefix;
mod x25519;

use std::ptr;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use base64::{engine::general_purpose::STANDARD, Engine as _};
use opencl3::command_queue::CommandQueue;
use opencl3::context::Context;
use opencl3::device::{get_all_devices, CL_DEVICE_TYPE_GPU};
use opencl3::kernel::{ExecuteKernel, Kernel};
use opencl3::memory::{Buffer, CL_MEM_READ_ONLY, CL_MEM_READ_WRITE};
use opencl3::program::Program;
use opencl3::types::{cl_uint, cl_ulong, CL_BLOCKING};

const KERNEL_SRC: &str = include_str!("../kernel.cl");
/// 每轮最多回收的命中数量（与 kernel 中 slot < 64 对应）
const MAX_HITS: usize = 64;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut prefix_arg: Option<String> = None;
    let mut count: usize = 1;
    let mut round_log2: u32 = 22; // 每轮 2^22 ≈ 4M 个候选
    let mut selftest: Option<usize> = None;

    let mut args = std::env::args().skip(1).peekable();
    while let Some(a) = args.next() {
        match a.as_str() {
            "--count" => count = args.next().ok_or("--count 需要参数")?.parse()?,
            "--round-log2" => round_log2 = args.next().ok_or("--round-log2 需要参数")?.parse()?,
            "--selftest" => {
                // 可选的数字参数：仅当下一个参数能解析为数字时消费它
                let n = args
                    .next_if(|s| s.parse::<usize>().is_ok())
                    .map(|s| s.parse().unwrap())
                    .unwrap_or(4096);
                selftest = Some(n);
            }
            "--help" | "-h" => {
                println!("用法: wg-vanity-opencl <前缀> [--count N] [--round-log2 N] [--selftest [N]]");
                return Ok(());
            }
            s if s.starts_with('-') => return Err(format!("未知参数 {s}").into()),
            s => prefix_arg = Some(s.to_string()),
        }
    }

    // ---- 初始化 OpenCL ----
    let devices = get_all_devices(CL_DEVICE_TYPE_GPU)?;
    if devices.is_empty() {
        return Err("没有找到 OpenCL GPU 设备".into());
    }
    let device = devices[0];
    let context = Context::from_device(&device.into())?;
    let queue = CommandQueue::create_default(&context, 0)?;
    let program = Program::create_and_build_from_source(&context, KERNEL_SRC, "-cl-std=CL1.2")?;

    if let Some(n) = selftest {
        return run_selftest(&context, &queue, &program, n);
    }

    let prefix = prefix_arg.ok_or("缺少前缀参数（--help 查看用法）")?;
    let pattern = prefix::from_prefix(&prefix)?;
    let estimate = 64f64.powi(prefix.len() as i32);
    println!("前缀 {prefix:?}：期望尝试次数 ≈ 2^{:.1}", estimate.log2());

    // ---- 准备 buffer ----
    let mut pat_buf = unsafe { Buffer::<u8>::create(&context, CL_MEM_READ_ONLY, 44, ptr::null_mut()) }?;
    let mut mask_buf = unsafe { Buffer::<u8>::create(&context, CL_MEM_READ_ONLY, 44, ptr::null_mut()) }?;
    let mut found_buf = unsafe { Buffer::<cl_uint>::create(&context, CL_MEM_READ_WRITE, 1, ptr::null_mut()) }?;
    let keys_buf =
        unsafe { Buffer::<cl_ulong>::create(&context, CL_MEM_READ_WRITE, MAX_HITS * 4, ptr::null_mut()) }?;

    unsafe { queue.enqueue_write_buffer(&mut pat_buf, CL_BLOCKING, 0, &pattern.pat, &[]) }?;
    unsafe { queue.enqueue_write_buffer(&mut mask_buf, CL_BLOCKING, 0, &pattern.mask, &[]) }?;

    let kernel = Kernel::create(&program, "vanity")?;
    let round_size = 1usize << round_log2;

    // 起始种子：纳秒时间戳，每次运行从不同的位置开始搜索
    let seed0 = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos() as u64;

    let mut total: u64 = 0;
    let mut collected = 0usize;
    let t_start = Instant::now();
    for round in 0u64.. {
        let seed = seed0.wrapping_add(round << round_log2);
        unsafe { queue.enqueue_write_buffer(&mut found_buf, CL_BLOCKING, 0, &[0u32], &[]) }?;

        unsafe {
            ExecuteKernel::new(&kernel)
                .set_arg(&seed)
                .set_arg(&pat_buf)
                .set_arg(&mask_buf)
                .set_arg(&(pattern.len as cl_uint))
                .set_arg(&found_buf)
                .set_arg(&keys_buf)
                .set_global_work_size(round_size)
                .enqueue_nd_range(&queue)
        }?;

        let mut found = [0u32; 1];
        unsafe { queue.enqueue_read_buffer(&found_buf, CL_BLOCKING, 0, &mut found, &[]) }?;
        total += round_size as u64;

        if found[0] > 0 {
            let n = (found[0] as usize).min(MAX_HITS);
            let mut words = vec![0u64; n * 4];
            unsafe { queue.enqueue_read_buffer(&keys_buf, CL_BLOCKING, 0, &mut words, &[]) }?;
            for slot in 0..n {
                let mut key = [0u8; 32];
                for i in 0..4 {
                    key[i * 8..i * 8 + 8]
                        .copy_from_slice(&words[slot * 4 + i].to_le_bytes());
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

/// GPU 与 curve25519-dalek 对拍 n 个派生密钥。
fn run_selftest(
    context: &Context,
    queue: &CommandQueue,
    program: &Program,
    n: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    let kernel = Kernel::create(program, "selftest_keys")?;
    let pubs_buf = unsafe { Buffer::<u8>::create(context, CL_MEM_READ_WRITE, n * 32, ptr::null_mut()) }?;

    let seed: u64 = 0xDEAD_BEEF_CAFE_F00D;
    unsafe {
        ExecuteKernel::new(&kernel)
            .set_arg(&seed)
            .set_arg(&pubs_buf)
            .set_global_work_size(n)
            .enqueue_nd_range(queue)
    }?;

    let mut pubs = vec![0u8; n * 32];
    unsafe { queue.enqueue_read_buffer(&pubs_buf, CL_BLOCKING, 0, &mut pubs, &[]) }?;

    for gid in 0..n {
        let key = keygen::derive_key(seed, gid as u64);
        let expect = curve25519_dalek::constants::X25519_BASEPOINT
            .mul_clamped(key)
            .to_bytes();
        let got = &pubs[gid * 32..gid * 32 + 32];
        if got != expect {
            return Err(format!(
                "自检失败：gid={} GPU={} 参考={}",
                gid,
                STANDARD.encode(got),
                STANDARD.encode(expect)
            )
            .into());
        }
    }
    println!("自检通过：{n} 个密钥与 curve25519-dalek 一致");
    Ok(())
}
