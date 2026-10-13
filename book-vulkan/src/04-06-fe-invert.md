# 4.6 fe.rs：invert

这节写域求逆——3.6 节加法链的直译。整段代码只有 `sq` 和 `mul` 两个原语，没有任何新数学。

## 代码

接在 `mul121665` 之后：

```rust
/// 逆元：z^(p-2) = z^(2^255 - 21)（费马小定理）。
/// 使用 ref10 的固定加法链，共 254 次平方 + 11 次乘法。
pub fn invert(z: &Fe) -> Fe {
    let t0 = sq(z); // z^2
    let mut t1 = sq(&t0);
    t1 = sq(&t1); // z^8
    t1 = mul(z, &t1); // z^9
    let t0 = mul(&t0, &t1); // z^11
    let t2 = sq(&t0); // z^22
    let t1 = mul(&t1, &t2); // z^31 = z^(2^5 - 1)

    let mut t2 = sq(&t1);
    for _ in 1..5 {
        t2 = sq(&t2);
    }
    let t1 = mul(&t2, &t1); // z^(2^10 - 1)

    let mut t2 = sq(&t1);
    for _ in 1..10 {
        t2 = sq(&t2);
    }
    let t2 = mul(&t2, &t1); // z^(2^20 - 1)

    let mut t3 = sq(&t2);
    for _ in 1..20 {
        t3 = sq(&t3);
    }
    let t2 = mul(&t3, &t2); // z^(2^40 - 1)

    let mut t2 = sq(&t2);
    for _ in 1..10 {
        t2 = sq(&t2);
    }
    let t1 = mul(&t2, &t1); // z^(2^50 - 1)

    let mut t2 = sq(&t1);
    for _ in 1..50 {
        t2 = sq(&t2);
    }
    let t2 = mul(&t2, &t1); // z^(2^100 - 1)

    let mut t3 = sq(&t2);
    for _ in 1..100 {
        t3 = sq(&t3);
    }
    let t2 = mul(&t3, &t2); // z^(2^200 - 1)

    let mut t2 = sq(&t2);
    for _ in 1..50 {
        t2 = sq(&t2);
    }
    let t1 = mul(&t2, &t1); // z^(2^250 - 1)

    let mut t1 = sq(&t1);
    for _ in 1..5 {
        t1 = sq(&t1);
    }
    mul(&t1, &t0) // z^(2^255 - 21)
}
```

## 读法

对照 3.6 节的滚雪球：每段的行尾注释标出当前累计指数，z^(2^5−1) → z^(2^10−1) → … → z^(2^250−1)，最后平方 5 次（指数变 2^255−32）再乘 t0（=z^11）得 z^(2^255−21)。

循环写成 `for _ in 1..N` 而不是 `0..N`：循环外已经平方过一次，循环里补 N−1 次，合计 N 次。敲的时候注意别改成 `0..N`，否则每段多平方一次，结果就全错了——这类错误单元测试（4.9 节）能立刻抓住。

💡 **补给：`let t0 = mul(&t0, &t1)` 的遮蔽（shadowing）。** Rust 允许同名再绑定，旧值被「遮蔽」。这里把早先的 z^2 乘进 z^11 得到新的 t0（z^11），后面的段还要用它。遮蔽让「每个中间结果只用一次」的链条读起来更清晰。

至此 `fe.rs` 的运算部分齐了：carry / add / sub / mul / sq / mul121665 / invert。下一节写最后两个函数：字节打包与解包。
