//! labcodec: Protobuf 编解码的极简封装
//!
//! 说明（中文注释）：
//! - 本 crate 是对 `prost` 的轻量封装，用于在课程实验中对 protobuf 消息进行
//!   编码与解码。上层 crate（如 `labrpc`、`raft`、`percolator`）通过路径依赖引用
//!   本库以统一消息序列化格式。
//! - 依赖说明（在 Cargo.toml 中）：
//!   - `prost` / `prost-derive`：负责从 `.proto` 生成 Rust 的消息类型（Message trait），
//!     并提供 `encode`/`decode` 等二进制序列化 API。
//!   - `prost-build`（build-dependency）：在构建时将 `.proto` 编译为 Rust 源码。
//!   - 上层 crate 通过 `include!(concat!(env!("OUT_DIR"), "/...rs"))` 将生成的文件
//!     包含进来，以便在编译期获得由 prost 生成的类型定义。
//!
//! 目标：保持接口最小并对 prost 类型做一点类型约束，便于上层代码以统一方式
//! 进行 `encode` / `decode` 调用。

/// `labcodec` 中可用作消息类型的统一 trait 定义（空 trait，仅做约束）
///
/// 目的与用法说明（逐项解释）：
/// - `pub trait Message: prost::Message + Default {}` 定义了一个新的 trait 名为 `Message`，
///   要求实现类型同时满足两个条件：
///   1. `prost::Message`：由 `prost` 提供的 trait，包含 `encode`、`encoded_len`、`decode` 等方法，
///      这些方法是序列化/反序列化 protobuf 消息所必需的。
///   2. `Default`：要求消息类型能提供一个默认值（用于测试与空输入解码场景）。
/// - 该 trait 本身不增加新方法，仅用于在 `encode` / `decode` 函数的泛型约束中使用，
///   使得上层调用时不必直接引用 `prost::Message`，而能使用 `labcodec::Message` 作为
///   项目内统一的消息类型别名。这样做利于将来对接口的扩展或替换实现。
pub trait Message: prost::Message + Default {}
impl<T: prost::Message + Default> Message for T {}

/// 将 `prost` 的错误类型在本库中暴露为更短且语义化的别名：`EncodeError`
///
/// - `prost::EncodeError`：在调用 `Message::encode` 序列化时可能产生的错误，通常与
///   序列化缓冲区或消息内部数据无效有关。
pub type EncodeError = prost::EncodeError;

/// 将 `prost` 的解码错误类型在本库中暴露为 `DecodeError`
///
/// - `prost::DecodeError`：在调用 `Message::decode` 反序列化二进制数据为消息时产生的错误，
///   例如输入数据不完整或不符合预期的 protobuf 格式。
pub type DecodeError = prost::DecodeError;

/// 将消息编码写入 `Vec<u8>` 的实用函数。
///
/// 详细说明：
/// - 参数 `message`: 实现了 `labcodec::Message` 的消息引用，代表要编码的 protobuf 结构体。
/// - 参数 `buf`: 用于写入编码后二进制数据的可变 `Vec<u8>`，函数不会分配新的 Vec，
///   而是将编码内容追加到 `buf` 上（调用方可复用缓冲区以减少分配）。
/// - 返回：`Result<(), EncodeError>`，成功返回 `Ok(())`，否则返回 prost 的 `EncodeError`。
///
/// 内部实现要点：
/// 1. 通过 `message.encoded_len()` 预留足够容量（`buf.reserve(...)`），以避免多次重新分配。
/// 2. 调用 `message.encode(buf)` 将二进制数据写入 `buf`。
pub fn encode<M: Message>(message: &M, buf: &mut Vec<u8>) -> Result<(), EncodeError> {
    // 预留足够的空间以减少内存重新分配。
    buf.reserve(message.encoded_len());
    // prost::Message::encode 将二进制数据追加到 provided buffer 中。
    message.encode(buf)?;
    Ok(())
}

/// 从二进制切片解码出一个消息实例。
///
/// 详细说明：
/// - 参数 `buf`: 包含 protobuf 编码数据的字节切片（通常来自网络或持久化存储）。
/// - 返回：`Result<M, DecodeError>`，成功时返回消息实例，失败时返回 `prost::DecodeError`。
///
/// 注意事项：调用方应保证 `buf` 的边界正确；对于空的 `buf`，如果消息类型有默认值，
/// `prost` 可能会返回默认实例（这取决于消息定义与 prost 的实现）。
pub fn decode<M: Message>(buf: &[u8]) -> Result<M, DecodeError> {
    M::decode(buf)
}

#[cfg(test)]
mod tests {
    mod fixture {
        // The generated rust file:
        // labs6824/target/debug/build/labcodec-hashhashhashhash/out/fixture.rs
        //
        // It looks like:
        //
        // ```no_run
        // /// A simple protobuf message.
        // #[derive(Clone, PartialEq, Message)]
        // pub struct Msg {
        //     #[prost(enumeration="msg::Type", tag="1")]
        //     pub type_: i32,
        //     #[prost(uint64, tag="2")]
        //     pub id: u64,
        //     #[prost(string, tag="3")]
        //     pub name: String,
        //     #[prost(bytes, repeated, tag="4")]
        //     pub paylad: ::std::vec::Vec<Vec<u8>>,
        // }
        // pub mod msg {
        //     #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Enumeration)]
        //     pub enum Type {
        //         Unknown = 0,
        //         Put = 1,
        //         Get = 2,
        //         Del = 3,
        //     }
        // }
        // ```
        include!(concat!(env!("OUT_DIR"), "/fixture.rs"));
    }

    use super::{decode, encode};

    #[test]
    fn test_basic_encode_decode() {
        let msg = fixture::Msg {
            r#type: fixture::msg::Type::Put as _,
            id: 42,
            name: "the answer".to_owned(),
            paylad: vec![vec![7; 3]; 2],
        };
        let mut buf = vec![];
        encode(&msg, &mut buf).unwrap();
        let msg1 = decode(&buf).unwrap();
        assert_eq!(msg, msg1);
    }

    #[test]
    fn test_default() {
        let msg = fixture::Msg::default();
        let msg1 = decode(&[]).unwrap();
        assert_eq!(msg, msg1);
    }
}
