# 7.8 常见报错速查

跟书过程中最常踩的五个坑，按报错信息对号入座。

## `无法运行 glslangValidator（请先安装 glslang）`

build.rs 找不到 shader 编译器，说明 glslang 没装或不在 PATH 里。回 2.3 节执行 `sudo pacman -S glslang`，用 `glslangValidator --version` 确认能打印版本号后重新 `cargo build`。

## `没有支持 shaderInt64 的 Vulkan 设备`

init() 枚举完所有物理设备都没找到支持 64 位整数 shader 的。回 2.3 节跑 `vulkaninfo --summary` 确认驱动认识你的显卡，再 `vulkaninfo | grep shaderInt64` 看该设备是否报告 `true`。如果是驱动没装好，装 `vulkan-radeon`；如果设备本身不支持，8.2 节的 32-bit limb 改造是退路。

## vulkano 一堆类型不存在 / 方法签名对不上

十有八九是 vulkano 版本不对——它的 API 几乎每个 minor 版本都有破坏性改动。检查 `Cargo.toml` 里是否真是 `vulkano = "0.35"`（见 5.4 节），改对后 `cargo build` 让依赖重新解析。不要顺手升级版本，网上抄来的代码片段也大概率是别的版本的。

## selftest 报 `自检失败：gid=N ...`

GPU 算出的公钥与 dalek 参考值不一致，说明 shader 的数学有移植错误。用 6.3 节的 `derive_key`（种子 `0xDEAD_BEEF_CAFE_F00D` + 报错信息里的 gid）手工复现出错的那个私钥，再逐函数对照第 6 章 GLSL 与第 4 章 Rust（见 7.6 节末尾的排查建议）。最常见的元凶：`UL` 后缀漏写、进位位数 26/25 抄反。

## build.rs panic：`glslangValidator 编译 shader.comp 失败`

glslang 找到了、也跑了，但 shader 本身编译不过。别只看 panic 那一行——往上翻，glslangValidator 自己会打印出错的**文件名和行号**（形如 `shader.comp:127: error: ...`），拿着行号对照第 6 章的代码块逐行核，基本都是抄字错误：漏分号、`UL` 后缀、括号不配对。如果 shader 编译通过、是自检对拍红了，排查思路见 7.6 节末尾。
