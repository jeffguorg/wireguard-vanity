# 3.1 WireGuard 密钥对与 base64

这节先玩一玩 WireGuard 官方工具生成密钥对，看清公钥私钥的长相，再搞懂 base64 编码——因为我们搜索的目标就是公钥的 base64 前缀。

## 生成一对密钥

装上官方工具：

```console
$ sudo pacman -S wireguard-tools
```

生成私钥、由私钥算出公钥：

```console
$ wg genkey
qBqT4dBop7VtUox6NUe0FrwMbbq/rXa9+zVdZYfAJGA=
$ echo 'qBqT4dBop7VtUox6NUe0FrwMbbq/rXa9+zVdZYfAJGA=' | wg pubkey
Sl2gmMieuzVw9dvJTwQkHNxyEZ8VOyFHk7UxLCYRxlg=
```

- `wg genkey`：随机生成 32 字节私钥，base64 输出；
- `wg pubkey`：从私钥算出公钥（就是 X25519，下节讲）。

注意两点：**两个 key 都是 44 个字符、以 `=` 结尾**；同一个私钥无论算多少次，公钥都一样——这是确定性计算，不是重新随机。

## base64 编码规则

base64 是把任意字节流转成「可打印字符」的编码：

- 字符表 64 个：`A-Z`（0–25）、`a-z`（26–51）、`0-9`（52–61）、`+`（62）、`/`（63）；
- 每个字符代表 **6 bit**；3 个字节（24 bit）正好拼成 4 个字符；
- 字节数不是 3 的倍数时，末尾用 `=` 补齐。

WireGuard 的 key 是 32 字节：32 × 8 = 256 bit，256 / 6 = 42.67，所以需要 43 个字符，再补 1 个 `=`，凑成 44 个字符。

> 💡 补给：43 × 6 = 258 bit，比 256 多出 2 bit，所以公钥第 43 个字符只能取 64 个值里的 16 个。搜前缀时不用管它——第 6 章的「掩码」机制天然只比较有效位。

## 为什么搜 base64 前缀而不是字节前缀

我们给人看的是 base64 字符串，「以 Hello 开头」指的就是编码后的字符串。但 GPU 算出来的是 32 字节的原始公钥，难道要在 GPU 上做 base64 编码再比较？不用——6 章会讲一个技巧：把 base64 前缀**预先翻译成字节级的「模式 + 掩码」**，GPU 直接比字节。

## 小结

- 密钥对 = 32 字节私钥 + 32 字节公钥，显示为 44 字符 base64；
- 我们的目标：找一个私钥，让它的公钥 base64 以指定前缀开头；
- `wg genkey` / `wg pubkey` 是官方实现，第 7 章会拿它给我们的程序做交叉验证。
