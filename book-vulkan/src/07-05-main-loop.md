# 7.5 主循环：push constants → dispatch → fence → 读结果

这节写 `main()`：参数解析、搜索分支的设置、以及一轮接一轮的主循环。把 7.1 以来占位的 `fn main() {}` 删掉，换成下面的真实实现。

## 参数解析与初始化

```rust
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
```

手搓的参数解析：`--count N` 要几个结果、`--round-log2 N` 每轮跑 2^N 个密钥（默认 2²² = 4M）、`--selftest [N]` 进自检模式（N 可省略，`next_if` 只在下一个参数是数字时才吃掉它）。`round_log2 = 22` 配 local_size 128：每轮 2²²/128 = 32768 个 workgroup。

> 📝 文件顺序提醒：接下来是 **selftest 分支**（7.6 节补上），然后才是下面的搜索分支。敲代码时先在 `mem_filter` 之后留出位置，或按 7.6 节说明最后补插。

## 搜索分支：设置

```rust
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
```

`prefix::from_prefix` 就是 6.2 节写的模块——前缀在这里变成 `pattern`（pat/mask/len）。buffer 长度 `1 + 64×4 = 257` 个 u64：word0 是计数，其后 64 槽 × 4 个 u64（6.7 节的布局）。`seed0` 用当前纳秒时间戳，每次运行搜不同的空间。

## 主循环

```rust
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
```

一轮的完整生命周期，正好串起全书的约定：

1. **清零计数**：`buf.write()?[0] = 0`（word0 低 32 位是 count，6.7 节）；
2. **组 push constant**：第 round 轮的种子 = seed0 + round×2^round_log2——每轮覆盖互不重叠的 gid 区间（6.3 节的不重复保证）；
3. **dispatch**：7.4 节的函数，里面包含 fence 等待，返回时结果已在 buffer 里；
4. **读结果**：`& 0xFFFF_FFFF` 取 count；命中槽里的 4 个 u64 按小端拼回 32 字节私钥；
5. **复核**：用 CPU 参考实现 `x25519_base` 重算公钥打印（顺带 `debug_assert!` 验证前缀匹配——release 下不执行，纯开发期保险）；
6. 每 10 轮往 stderr 报一次进度；凑够 `--count` 个就收工。

`for round in 0u64..` 是无穷 range 迭代器，唯一的出口是 `break`（或 Ctrl-C）。

## 收尾：统计打印与返回

主循环之后，补上 main 的最后几行：

```rust
    let secs = t_start.elapsed().as_secs_f64();
    eprintln!(
        "完成：共尝试 {} 次，耗时 {:.1}s，平均 {:.1} M/s",
        total,
        secs,
        total as f64 / secs / 1e6
    );
    Ok(())
}
```

`main` 返回 `Ok(())` 时进程正常退出；任何一步的 `Err` 都会被 Rust 运行时打印出来并以非零码退出。统计打印走 stderr，和循环里的进度报告同一通道。

至此 main 整体成型，`cargo run --release -- --help` 应该已经能编译通过。先别急着跑搜索——下一节补上 selftest 分支，先让 GPU 自证正确。
