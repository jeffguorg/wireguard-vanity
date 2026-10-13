//! 构建脚本：用 glslangValidator 把 shader.comp 编译成 SPIR-V。
//! 同一份源码编译两次：正常版（vanity）与自检版（-DSELFTEST）。

use std::process::Command;

fn main() {
    let out_dir = std::env::var("OUT_DIR").unwrap();
    for (name, define) in [("shader", ""), ("selftest", "SELFTEST")] {
        let dst = format!("{out_dir}/{name}.spv");
        let mut cmd = Command::new("glslangValidator");
        cmd.args(["-V", "--target-env", "vulkan1.1", "shader.comp", "-o", &dst]);
        if !define.is_empty() {
            cmd.arg(format!("-D{define}"));
        }
        let status = cmd
            .status()
            .expect("无法运行 glslangValidator（请先安装 glslang）");
        assert!(status.success(), "glslangValidator 编译 {name} 失败");
    }
    println!("cargo:rerun-if-changed=shader.comp");
}
