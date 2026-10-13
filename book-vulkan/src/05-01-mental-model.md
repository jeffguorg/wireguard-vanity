# 5.1 Vulkan 的心智模型

第 5 章不写项目代码，用四节把 Vulkan 的核心概念讲透，第 7 章的宿主程序就只是「把这些概念翻译成 vulkano 调用」。这节先建立对象层级的心智模型。

## 四层对象

```text
Instance（实例）
  └─ PhysicalDevice（物理设备，每块显卡一个）
       └─ Device（逻辑设备，你向驱动「开户」得到的句柄）
            └─ Queue（队列，提交工作的传送带）
```

- **Instance**：进程与 Vulkan 驱动之间的总入口。创建它 = 加载驱动、声明要用哪些扩展。一个进程通常只有一个。
- **PhysicalDevice**：枚举出来的真实硬件。可以查询它的型号、支持的特性（比如我们的 `shaderInt64`）、队列族。
- **Device**：基于某块物理设备创建的「逻辑设备」，相当于开户：声明「我要用哪些特性、哪些队列」。之后所有资源（buffer、pipeline）都挂在它名下。
- **Queue**：真正干活的地方。GPU 工作不是「调用函数」，而是「把命令包提交到队列」。队列分族（graphics / compute / transfer），我们只需要支持 compute 的族。

## 和 OpenCL 对照

熟 OpenCL 的话一句话就够：`platform+device` ≈ instance+physical device，`context` ≈ device，`command queue` ≈ queue。差别在于 Vulkan 什么都显式：没有默认上下文，内存类型、同步、命令缓冲全要自己搭——啰嗦，但没有隐藏魔法。

## 工作提交流程（先有个印象）

一次 GPU 计算的完整流程，对应第 7 章 `dispatch` 函数的每一步：

```text
1. 录命令：bind pipeline → bind descriptor set → push constants → dispatch
2. 命令打包成 command buffer，提交到 queue
3. GPU 执行（异步，CPU 继续跑自己的）
4. fence 变绿，CPU 知道「算完了」，去读结果 buffer
```

**fence**（栅栏）是 GPU→CPU 的单向信号：提交时让它挂在命令后面，GPU 执行完就把它置位；CPU 调用 `wait` 阻塞等它。

## 为什么设计得这么显式

Vulkan 的设计哲学是「驱动不猜」。OpenCL/老 OpenGL 里驱动替你管理内存、同步、状态缓存，方便但 unpredictable；Vulkan 把这些责任全部交给应用，换来驱动的轻薄和性能的可预测。对我们这种「同一个 kernel 反复跑」的程序，初始化啰嗦一次，主循环反而干净。
