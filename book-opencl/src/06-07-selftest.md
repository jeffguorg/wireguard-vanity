# 6.7 selftest_keys 入口

这节写 kernel 的第二个入口 `selftest_keys`：不算命中、不比前缀，把每个候选的公钥原样写回显存，让宿主端逐字节对拍。它是 GPU 正确性的试金石（7.4 节使用）。

## 代码

文件末尾接上，`kernel.cl` 就此完整：

```c
// 自检：把每个候选的公钥原样写出，宿主端与参考实现比对
__kernel void selftest_keys(u64 seed, __global u8 *pubs) {
    u64 gid = (u64)get_global_id(0);
    u64 w[4];
    derive_key(seed, gid, w);
    u8 pub[32];
    wg_pubkey(w, pub);
    for (int i = 0; i < 32; i++) pubs[gid * 32 + i] = pub[i];
}
```

逻辑比 `vanity` 还简单：派生 → 算公钥 → 按 gid 写进 `pubs` 数组的对应 32 字节。发射 N 个 work-item 就得到 N 个 GPU 算出的公钥。

## 为什么要单独一个入口

直接在 `vanity` 里自检有两个麻烦：

- vanity 只在**命中**时写回，想验证得先等命中——验证速度受前缀长度绑架；
- 命中的公钥拿回来了，但你没法区分「GPU 算错了」和「宿主比对代码写错了」。

独立入口把两件事解耦：`selftest_keys` 无条件下输出**全部**公钥，宿主端用 dalek 逐个比对（固定 seed = 0xDEAD_BEEF_CAFE_F00D，结果完全确定，7.4 节）。任何 bit 级错误都会在几千个样本里现形。

## kernel.cl 完工检查

到这里 `kernel.cl` 应该是完整的：文件头 → sm64_next → fe_* → fe_invert → fe_tobytes → cswap → wg_pubkey → derive_key → vanity → selftest_keys，330 余行。和第 6 章开头的文件地图对一遍，缺哪段回哪节补。

它还**不能被单独编译运行**——OpenCL kernel 要等宿主程序在运行时喂给驱动编译。这正是下一章的内容。

## 小结

kernel.cl 全部写完：两个入口 + 一整套域运算。下一章写宿主程序，把它跑起来。
