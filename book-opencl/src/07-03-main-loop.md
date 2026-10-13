# 7.3 主循环：写参数 → 发射 → 读结果 → 打印速率

这节写主循环：每轮清计数器、发射 2²² 个 work-item、读回命中、打印进度。main 函数在本节收尾。

## 代码

接在 `seed0` 那行后面：

```rust
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
```

## 逐段讲

**seed 调度**：第 round 轮的 seed = `seed0 + (round << round_log2)`。一轮的 gid 覆盖 [0, 2²²)，派生状态是 `seed + gid·φ`……严格说各轮之间用的是 seed 错开的 splitmix64 序列，时间戳起点 + 每轮偏移 2²²，反复运行、多轮推进都**几乎不会**重复摸同一批候选。

说「几乎」是因为：两轮的状态集合 `{seed + gid·φ}` 要撞上，需要 seed 之差恰好是 φ 的某个小倍数（< 2²²）——seed 由时间戳随机起步，跨轮撞上的概率在 2⁻⁴⁰ 量级，可以忽略。想做到绝对不相交？留作练习题（提示：构造互不重叠的序列段）。

**每轮开头清计数器**：`&[0u32]` 写回 found——kernel 的原子计数是累加的，每轮必须归零。

**ExecuteKernel 链**：6 个 `set_arg` 的顺序与 `vanity` 的 6 个参数**严格一一对应**（5.4 节的警告在此兑现）：seed↔u64、pat_buf↔`__constant u8*`、mask_buf、patlen↔u32、found_buf↔`__global u32*`、keys_buf↔`__global u64*`。类型错了不报错只算错，核对了再往下走。

**隐式同步**：`enqueue_nd_range` 是异步的（入队就返回），但紧接着的 `enqueue_read_buffer` 带 `CL_BLOCKING`，队列内按序执行——读计数器时 kernel 必然已跑完。不需要手动 finish。

**命中处理**：读回 keys 后用**我们自己的 CPU x25519** 重算公钥、用 `debug_assert!` 再验一次前缀（4.9 节的测试资产在这里继续站岗），然后 base64 打印。攒够 `count` 个就跳出。注意 `count` 是「攒够就停」的下限：短前缀可能一轮就命中打满 64 个 slot（6.6 节），这时 `--count 3` 实际会打印 64 个密钥，不是 3 个。

**速率打印**：每 10 轮往 **stderr** 打一次进度（`eprintln!`），密钥走 stdout——这样 `./wg-vanity-opencl Hi > keys.txt` 重定向时进度不会混进结果文件。

> 💡 补给：`debug_assert!` 只在 debug 构建生效，release 下编译为零开销——「验证不花生产环境的钱」。`Instant::now()` 是单调时钟，测耗时专用（不受系统时间调整影响）。

## 小结

main 函数完成。跑长前缀前，先用下节的 selftest 确认 GPU 算的每一个密钥都对。
