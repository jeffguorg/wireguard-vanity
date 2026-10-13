# 7.4 selftest：与 dalek 对拍

这节写 `run_selftest` 函数：让 GPU 批量算公钥，宿主端用 keygen.rs + dalek 重建同样的候选并逐字节比对。三层验证的第二层。

## 代码

接在 main 函数的 `}` 后面（文件末尾）：

```rust
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
```

## 链条复盘

这段代码把前六章所有东西串成了一条验证链：

```text
kernel.cl:  derive_key(seed, gid) ──► wg_pubkey ──► pubs[gid]
                 ║ 必须逐位一致（6.2 节的承诺）
keygen.rs:  derive_key(seed, gid) ──► dalek.mul_clamped ──► expect
```

- 固定 seed `0xDEAD_BEEF_CAFE_F00D`：自检完全确定，可复现；
- 比对的是 **dalek**（第三方成熟实现），不是我们 CPU 版——三方对拍，任何一方错了都会暴露；
- `mul_clamped(key)` 内部会自己做 clamp，而 GPU 的 derive_key 也 clamp 过——等价是因为 clamp 幂等（对已 clamp 的输入再 clamp 不变）。

## 跑起来

```console
$ cargo run --release -- --selftest 8192
自检通过：8192 个密钥与 curve25519-dalek 一致
```

看到这行字，说明 kernel.cl 里从 splitmix64 到梯子到求逆**每一个 bit 都对**。默认不带数字是对拍 4096 个。

> 💡 补给：`pubs[gid*32..gid*32+32]` 是切片范围语法，取第 gid 个 32 字节窗口；`got != expect` 里切片和数组比较靠 Rust 的 PartialEq 自动适配。

## 小结

GPU 实现被 8192 个样本钉死。下节跑真实搜索并用 `wg pubkey` 做最后一层验证。
