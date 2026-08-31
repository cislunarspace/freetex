fn main() {
    // Linux：编译 C23 兼容符号 shim（见 c/isoc23_shim.c 注释），让 pyke 预编译的
    // onnxruntime 静态库能链接到 glibc 2.35（Ubuntu 22.04）。
    // Linux: build the C23 compat-symbol shim (see c/isoc23_shim.c) so pyke's
    // prebuilt onnxruntime static archives link against glibc 2.35 (Ubuntu 22.04).
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target_os == "linux" {
        println!("cargo:rerun-if-changed=c/isoc23_shim.c");
        // +whole-archive：GNU ld（aarch64 默认）按序拉静态档成员，顺序敏感；
        // 整档并入保证转发函数无条件参与链接（体积可忽略）。
        // +whole-archive: GNU ld (default on aarch64) pulls archive members by
        // order; including the whole (tiny) archive makes the forwarding
        // definitions order-independent.
        cc::Build::new()
            .file("c/isoc23_shim.c")
            .link_lib_modifier("+whole-archive")
            .compile("freetex_isoc23_shim");
    }
    tauri_build::build()
}
