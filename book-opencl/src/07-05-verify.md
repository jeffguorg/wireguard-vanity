# 7.5 输出与 wg pubkey 交叉验证

这节跑第一次真实搜索，并用 WireGuard 官方工具做第三层、也是最终一层验证。

## 第一次搜索

先来个短的，秒出结果：

```console
$ cargo run --release -- Hi
前缀 "Hi"：期望尝试次数 ≈ 2^12.0
私钥: FCO51GufpOG4j4jGgKel3h3xVMbvccSpLeO/v6BjB0I=
公钥: HilR2hg5E1h/1W0aarTYEWZteZQSP5vZNqiagafgIEY=
已尝试 0.00e9 次，速率 ...
```

（你的具体密钥会不一样——搜索起点是时间戳随机化的。）

要点：**公钥确实以 `Hi` 开头**。进度和速率走 stderr，密钥对走 stdout，可以安全重定向：

```console
$ cargo run --release -- Hello --count 3 > keys.txt
```

## wg pubkey 交叉验证（第三层）

程序自己已经用 CPU 实现复查过（7.3 节的 debug_assert），但「我们的 GPU 和我们的 CPU 犯同一个错」的可能性要用一个**完全独立的第三方**排除——WireGuard 官方工具：

```console
$ echo 'FCO51GufpOG4j4jGgKel3h3xVMbvccSpLeO/v6BjB0I=' | wg pubkey
HilR2hg5E1h/1W0aarTYEWZteZQSP5vZNqiagafgIEY=
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
PrivateKey = FCO51GufpOG4j4jGgKel3h3xVMbvccSpLeO/v6BjB0I=
...
```

对端配置里写你搜到的公钥。老规矩：私钥不外传，`keys.txt` 注意权限（`chmod 600`）。

## 小结

程序完工且三层验证闭环。下一章看实测性能数字，以及想更快/更好玩可以往哪些方向改。
