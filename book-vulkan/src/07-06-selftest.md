# 7.6 selftest：与 dalek 对拍

这节补上 `main()` 里的自检分支，并跑通它——这是 GPU 代码正确性的最终裁决，跑搜索前必须全绿。

## 代码

插在 7.5 节的 `mem_filter` 定义之后、搜索分支之前：

```rust
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
```

## 逻辑解读

- **加载的是 SELFTEST_SPV**（shader 里 `#ifdef SELFTEST` 的那支，6.1 节）：每个线程把完整公钥写进 `pubs`，不做前缀匹配；
- `n.next_multiple_of(128)`：workgroup 是 128 线程一组，n 不是 128 的倍数时最后一组会越组写入，向上取整避免越界；
- 固定种子 `0xDEAD_BEEF_CAFE_F00D`：自检要可复现，不用时间戳；
- dispatch 完成后逐 gid 比对：GPU 写的公钥 vs `derive_key` 派生私钥经 **dalek** 算出的公钥。注意裁判是 dalek 而不是我们自己的 `x25519_base`——dalek 是独立实现，对拍才有意义（反正 4.9 节已证明两者一致）；
- 任一不一致：打印 gid、GPU 结果、参考结果后退出——有了 gid 就能定位是哪个私钥算错。

## 跑起来

```console
$ cargo run --release -- --selftest 8192
使用设备: AMD Radeon RX 7900 XTX (RADV NAVI31, LLVM ...)
自检通过：8192 个密钥与 curve25519-dalek 一致
```

作者实测 8192 个密钥全部一致。这一步绿了，说明 shader 里的域运算、梯子、求逆、打包、buffer 回传、字节序——整条链全部正确。

如果红了怎么办：先用小 N（如 `--selftest 128`）缩小范围；失败信息里的 gid 配合 6.3 节的 `derive_key` 可以手工复现那个私钥，再回头逐函数对照第 6 章的 GLSL 与第 4 章的 Rust。最常见的移植错误：`UL` 后缀漏写（6.3 节）、进位位数抄错（26/25 抄反）。其他报错的速查表见 7.8 节。
