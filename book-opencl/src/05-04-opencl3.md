# 5.4 opencl3 crate 速览

这节认识我们在 Rust 侧用的库 `opencl3`：它提供哪些类型、为什么几乎每个调用都是 `unsafe`。

## opencl3 是什么

`opencl3` 是 OpenCL 3.0 C API 的 Rust 封装，结构很薄：每个 Rust 类型包一个 C 句柄，每个方法转调一个 C 函数。我们只用到下面 7 行 import（最终 main.rs 里 opencl3 部分的完整 import 列表）：

```rust
use opencl3::command_queue::CommandQueue;
use opencl3::context::Context;
use opencl3::device::{get_all_devices, CL_DEVICE_TYPE_GPU};
use opencl3::kernel::{ExecuteKernel, Kernel};
use opencl3::memory::{Buffer, CL_MEM_READ_ONLY, CL_MEM_READ_WRITE};
use opencl3::program::Program;
use opencl3::types::{cl_uint, cl_ulong, CL_BLOCKING};
```

对应关系：

| 类型 | 角色 | 在哪用 |
|---|---|---|
| `get_all_devices` | 枚举 GPU 设备 | 7.2 |
| `Context` / `CommandQueue` | 上下文 / 队列 | 7.2 |
| `Program` | 从源码编译 kernel 程序 | 7.2 |
| `Kernel` | program 里的一个入口函数 | 7.2 |
| `Buffer` | 显存 buffer | 5.3、7.2 |
| `ExecuteKernel` | 绑参数 + 发射的建造者 | 7.3 |
| `cl_uint` / `cl_ulong` | C 侧类型别名（u32/u64） | 7.2 |

## 为什么全是 unsafe

你会看到宿主代码里到处都是 `unsafe { ... }`：

```rust
unsafe { queue.enqueue_write_buffer(&mut pat_buf, CL_BLOCKING, 0, &pattern.pat, &[]) }?;
```

原因：OpenCL 是 C API，Rust 编译器无法检查它的安全前提。比如 `enqueue_read_buffer` 接受一个裸指针和长度，「指针指向的内存够不够大」只有程序员知道；kernel 参数的类型和个数对不对，编译期也无从得知——绑错类型就是显存里读垃圾。

> 💡 补给：`unsafe { }` 块的意思是「这里面的操作编译器担保不了，我以人头担保」。它**不是**「关掉检查乱来」，而是把责任从编译器转移到我们身上。写完每处 unsafe 都问自己：指针有效吗？长度对吗？类型匹配吗？

## ExecuteKernel：建造者模式

发射 kernel 的写法是链式的（7.3 节细讲）：

```rust
ExecuteKernel::new(&kernel)
    .set_arg(&seed)
    .set_arg(&pat_buf)
    // …
    .set_global_work_size(round_size)
    .enqueue_nd_range(&queue)
```

`set_arg` 按 kernel 参数顺序逐个绑定，`set_global_work_size` 定规模，最后 `enqueue_nd_range` 入队。Rust 侧 `set_arg(&x)` 传的类型必须和 kernel.cl 里声明的参数类型**逐一对应**——对不上不会报错，只会算错，这也是第 7 章要小心的地方。

## 小结

opencl3 = OpenCL C API 的薄封装；unsafe 多是因为安全检查只能靠人；类型对应关系是暗坑，7.3 节我们逐个核对。
