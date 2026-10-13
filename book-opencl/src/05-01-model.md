# 5.1 平台/设备/上下文/命令队列的心智模型

这节建立 OpenCL 主机侧的四个核心概念。不急着写代码——第 7 章每一行宿主代码都对应这里的一个概念。

## 四层概念

```text
平台 Platform     ← 「哪家厂商的实现」（AMD APP Platform）
  └─ 设备 Device   ← 「哪块卡」（gfx1100）
       └─ 上下文 Context  ← 「这次工作的容器」：管内存、管 program
            └─ 命令队列 CommandQueue  ← 「任务单」：发射 kernel、搬数据都排进这里
```

逐个说：

- **平台（Platform）**：一个 OpenCL 实现。2.4 节 `clinfo` 里的 "AMD Accelerated Parallel Processing" 就是一个平台；
- **设备（Device）**：平台下的一块可计算硬件。我们用 `CL_DEVICE_TYPE_GPU` 过滤出显卡；
- **上下文（Context）**：绑定设备后创建的工作环境，之后分配的 buffer、编译的 program 都挂在它名下；
- **命令队列（CommandQueue）**：发给 GPU 的所有任务（写数据、跑 kernel、读数据）都塞进队列，GPU 按序消化。

## 一个类比

把 GPU 想成一家餐厅后厨：平台是餐饮集团，设备是具体那家店的厨房，上下文是你订的包间（你的菜、你的食材都在里面），命令队列是你点的菜单——你只管往上写菜名，厨房按顺序做。你（宿主程序）不直接下厨。

## 对应到代码

第 7 章的初始化四行，正好一行一个概念：

```rust
let devices = get_all_devices(CL_DEVICE_TYPE_GPU)?;      // 列出所有 GPU 设备
let device = devices[0];                                  // 用第一块
let context = Context::from_device(&device.into())?;      // 建上下文
let queue = CommandQueue::create_default(&context, 0)?;   // 建队列
```

（这是最终 main.rs 里的真实代码，这里先混个眼熟，7.2 节细讲。）

> 💡 补给：`?` 是 Rust 的错误传播语法：`expr?` 在出错时直接让当前函数返回这个错误，等价于一个 `match`。main 函数返回 `Result` 时，错误会打印出来并以非零码退出。

## 小结

平台→设备→上下文→队列，四层各建一次，之后所有操作都通过队列下达。下一节看 GPU 那边：kernel 和 work-item。
