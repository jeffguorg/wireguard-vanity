# 7.5 输出与 wg pubkey 交叉验证

这节跑第一次真实搜索，并用 WireGuard 官方工具做第三层、也是最终一层验证。

## 第一次搜索

先来个短的，秒出结果：

```console
$ cargo run --release -- MiAn
前缀 "MiAn"：期望尝试次数 ≈ 2^24.0
私钥: sJoNlMe3Ulc0egP1M8TAPx+3JDQw7H7xL917ocTg+m8=
公钥: MiAn2DjnLQR3YhKlV4B0TZ60Y261ZUdfJFWewHdeyA4=

私钥: kGqw61HnvNuyZLME54oTwW0L9GsX96BTAU57rvSKNG0=
公钥: MiAnfq6FNXmB2Phou7aZnQYrqdgtD3Q7bDtXRXUrRE8=

完成：共尝试 8388608 次，耗时 0.8s，平均 10.2 M/s
```

（你的具体密钥会不一样——搜索起点是时间戳随机化的。这次跑了两轮、撞上一对半：第二轮命中两个，都打印出来了。）

4 字符前缀每轮期望命中约 0.25 个（一轮 2²² 个候选、命中率 1/2²⁴），通常几轮内就能出一对；而 2 字符前缀一轮期望命中约 1024 个，会打满 64 个 slot 刷屏（见 7.3）。

要点：**公钥确实以 `MiAn` 开头**。进度和速率走 stderr，密钥对走 stdout，可以安全重定向：

```console
$ cargo run --release -- Hello --count 3 > keys.txt
```

## wg pubkey 交叉验证（第三层）

程序打印的公钥本身就是用我们的 CPU 实现从私钥重算的；debug 构建下 7.3 节的 `debug_assert` 还会额外断言一次前缀匹配（release 下不编译）。但「我们的 GPU 和我们的 CPU 犯同一个错」的可能性要用一个**完全独立的第三方**排除——WireGuard 官方工具：

```console
$ echo 'sJoNlMe3Ulc0egP1M8TAPx+3JDQw7H7xL917ocTg+m8=' | wg pubkey
MiAn2DjnLQR3YhKlV4B0TZ60Y261ZUdfJFWewHdeyA4=
```

`wg pubkey` 输出的公钥和程序打印的逐字符一致——三层验证闭环：

| 层 | 验证什么 | 裁判 |
|---|---|---|
| `cargo test` | CPU 实现的 X25519 | RFC 7748 + dalek |
| `--selftest` | GPU kernel 的每个 bit | dalek（8192 密钥） |
| `wg pubkey` | 最终产出的私钥可用 | WireGuard 官方工具 |

## 私钥怎么用

搜到的私钥就是标准 WireGuard 私钥，填进配置即可：

```ini
[Interface]
PrivateKey = sJoNlMe3Ulc0egP1M8TAPx+3JDQw7H7xL917ocTg+m8=
...
```

对端配置里写你搜到的公钥。老规矩：私钥不外传，`keys.txt` 注意权限（`chmod 600`）。

## 小结

程序完工且三层验证闭环。下一章看实测性能数字，以及想更快/更好玩可以往哪些方向改。
