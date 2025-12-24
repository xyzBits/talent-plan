# labcodec — Protobuf 编解码封装（中文说明）

## 项目目的

`labcodec` 是本课程（Distributed Systems in Rust）中用于处理 protobuf 消息的最小封装库。它基于 `prost` 进行二进制序列化/反序列化，并在构建阶段通过 `prost-build` 将 `proto/` 下的 `.proto` 文件编译为 Rust 源码。上层子项目（如 `labrpc`、`raft`、`percolator`）通过路径依赖使用本库生成的类型与辅助函数，以实现一致的消息编解码格式。

## 关键职责
- 在构建阶段把 `.proto` 编译为 Rust（生成文件放在 `OUT_DIR`）。
- 提供统一的、极简的 `encode` / `decode` 接口，封装 `prost` 的方法，便于上层项目复用。
- 包含一个示例/测试用的 proto（`proto/fixture.proto`）和对应的生成文件示例（由 `build.rs` 触发生成）。

## 依赖说明（以及用途）
- `prost` / `prost-derive`：核心依赖，用于定义和实现 protobuf 消息的 Rust 表示（自动派生 `Message` trait），并提供 `encode`/`decode` API。
- `prost-build`（build-dependency）：在构建期间将 `.proto` 文件编译成 Rust 源码（放入 `OUT_DIR`），以便在编译时通过 `include!` 引入。

注意：通常只需要安装 Rust 工具链（`rustup`、`cargo`）。`prost-build` 在大多数情况下可以自动处理生成步骤；若构建失败，可能需要安装系统的 `protoc`（Protocol Buffers 编译器），请按错误提示安装。

## 源码要点（包含在文档中的重要代码片段）

下面摘录本模块的核心代码并逐行说明，帮助理解 `labcodec` 的实现。

- `labcodec/src/lib.rs` 的主要内容：

```rust
// 用于约束上层消息类型的 trait，要求同时实现 prost::Message 与 Default
pub trait Message: prost::Message + Default {}
impl<T: prost::Message + Default> Message for T {}

// 将 prost 的错误类型在本库中简化为别名
pub type EncodeError = prost::EncodeError;
pub type DecodeError = prost::DecodeError;

// 将消息编码追加到提供的 Vec<u8> 中
pub fn encode<M: Message>(message: &M, buf: &mut Vec<u8>) -> Result<(), EncodeError> {
    buf.reserve(message.encoded_len());
    message.encode(buf)?;
    Ok(())
}

// 从字节切片解码出消息实例
pub fn decode<M: Message>(buf: &[u8]) -> Result<M, DecodeError> {
    M::decode(buf)
}
```

说明要点：
- `Message` trait 只是一个别名/约束，便于把 `prost::Message + Default` 封装成项目内部统一的消息类型接口。
- `encode`/`decode` 是对 `prost` 的简单包装，`encode` 支持将结果追加到已有缓冲区以复用内存。

- `labcodec/build.rs`（构建时生成 proto 代码）：

```rust
fn main() {
    prost_build::compile_protos(&["proto/fixture.proto"], &["proto"]).unwrap();
    println!("cargo:rerun-if-changed=proto");
}
```

说明要点：
- `prost_build::compile_protos` 会把 `proto/fixture.proto` 编译为 Rust 代码并输出到 `OUT_DIR`。
- `println!("cargo:rerun-if-changed=proto")` 告诉 Cargo 如果 `proto` 目录发生变化则重新运行 `build.rs`，从而重新生成代码。

- `labcodec/proto/fixture.proto`（示例 proto 文件）：

```proto
syntax = "proto3";
package fixture;

message Msg {
    enum Type { UNKNOWN = 0; PUT = 1; GET = 2; DEL = 3; }
    Type type = 1;
    uint64 id = 2;
    string name = 3;
    repeated bytes paylad = 4;
}
```

说明：该 proto 是一个最小示例，用于测试 prost 生成与本库的 encode/decode 函数。

## 如何编译与测试 `labcodec` 子项目（命令行）

以下命令在 Windows / PowerShell 下均可运行（前提：已安装 Rust 工具链）。

- 在工作区根目录单独构建 `labcodec`：

```powershell
cargo build --manifest-path labcodec/Cargo.toml
```

构建时 `build.rs` 会运行，生成 `OUT_DIR` 下的 Rust 文件（由 `prost-build` 生成）。

- 运行 `labcodec` 的测试：

```powershell
cargo test --manifest-path labcodec/Cargo.toml
```

- 如果要强制重新生成 proto（clean 然后构建）：

```powershell
cargo clean --manifest-path labcodec/Cargo.toml
cargo build --manifest-path labcodec/Cargo.toml
```

- 若构建过程中出现 prost/protoc 相关错误，请查看错误信息：可能需要在系统上安装 `protoc`，或在 `prost-build` 中打开 vendored 选项（视项目配置而定）。

## 使用示例（在上层项目中如何使用 labcodec 生成的类型）

1. 构建时 `labcodec` 的 `build.rs` 会生成 `OUT_DIR/fixture.rs`，上层代码可通过以下方式包含生成的类型：

```rust
include!(concat!(env!("OUT_DIR"), "/fixture.rs"));
```

2. 得到生成的类型后，可以直接使用我们的 `encode` / `decode`：

```rust
let msg = fixture::Msg::default();
let mut buf = Vec::new();
labcodec::encode(&msg, &mut buf).unwrap();
let msg2: fixture::Msg = labcodec::decode(&buf).unwrap();
```

## 调试提示
- 在运行测试或构建出现问题时，可以开启更详细的日志查看 `prost-build` 的输出：

```powershell
# 在 PowerShell 中设置环境变量并运行（示例）
$env:RUST_LOG = "debug"
cargo test --manifest-path labcodec/Cargo.toml
```

- 如果遇到 `protoc` 找不到或版本不兼容的错误，尝试安装系统的 Protocol Buffers 编译器，或按相关 crate 的文档启用 vendored 编译器。

## 文件位置
- 源码：`labcodec/src/lib.rs`
- 构建脚本：`labcodec/build.rs`
- 示例 proto：`labcodec/proto/fixture.proto`
- 中文 README（本文件）：`labcodec/README_zh.md`

---

需要我把 `labcodec` 的其它文件（例如生成的 `OUT_DIR/fixture.rs` 的示例展开内容）也写进文档，或者为 `labrpc` 添加类似的详尽注释和 README 吗？