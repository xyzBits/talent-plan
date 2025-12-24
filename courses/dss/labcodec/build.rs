// build.rs: 在构建（cargo build）阶段调用，用来将 proto 定义编译为 Rust 源码。
//
// 说明（中文注释）：
// - 该文件由 Cargo 自动在构建时执行，目的是通过 `prost-build` 将 `proto/` 目录中的
//   `.proto` 文件转换为 Rust 代码并输出到构建时的 `OUT_DIR`。上层代码随后通过
//   `include!(concat!(env!("OUT_DIR"), "/filename.rs"))` 将生成的 Rust 文件包含进来。
// - 这是 protobuf -> Rust 的生成步骤，确保消息类型在编译期可用而无需手写序列化代码。
// - 如果你修改了 `.proto` 文件，`cargo` 会在下次构建时重新运行此脚本来更新生成文件。

fn main() {
    // 将指定的 proto 文件编译，第二个参数是 proto 引用的搜索路径。
    // 这里把 proto/fixture.proto 编译为 Rust，并将生成文件放到 `OUT_DIR`。
    prost_build::compile_protos(&["proto/fixture.proto"], &["proto"]).unwrap();

    // 通知 Cargo：如果 proto 目录下内容变化，重新运行 build 脚本。
    // `cargo:rerun-if-changed=...` 是 build 脚本与 cargo 的约定输出，用于决定何时重新运行 build.rs
    println!("cargo:rerun-if-changed=proto");
}
