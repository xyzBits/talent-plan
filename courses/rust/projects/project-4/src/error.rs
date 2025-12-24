use failure::Fail;
use std::io;
use std::string::FromUtf8Error;

/// Error type for kvs
#[derive(Fail, Debug)]
pub enum KvsError {
    /// IO error
    #[fail(display = "IO error: {}", _0)]
    Io(#[cause] io::Error),
    /// Serialization or deserialization error
    #[fail(display = "serde_json error: {}", _0)]
    Serde(#[cause] serde_json::Error),
    /// Removing non-existent key error
    #[fail(display = "Key not found")]
    KeyNotFound,
    /// Unexpected command type error.
    /// It indicated a corrupted log or a program bug.
    #[fail(display = "Unexpected command type")]
    UnexpectedCommandType,
    /// Key or value is invalid UTF-8 sequence
    #[fail(display = "UTF-8 error: {}", _0)]
    Utf8(#[cause] FromUtf8Error),
    /// Sled error
    #[fail(display = "sled error: {}", _0)]
    Sled(#[cause] sled::Error),
    /// Error with a string message
    #[fail(display = "{}", _0)]
    StringError(String),
}

// 详细中文注释（补充）：
// 1. 设计目的：`KvsError` 封装了本仓库可能遇到的主要错误类型，包含底层 I/O、序列化、第三方库错误以及业务错误（如 key not found）。
// 2. 每个变体含义：
//    - `Io`：底层文件或网络 I/O 错误。
//    - `Serde`：JSON 序列化/反序列化相关错误。
//    - `KeyNotFound`：业务语义错误，表示尝试删除或访问不存在的 key。
//    - `UnexpectedCommandType`：当从日志中读取到了不符合预期的命令类型，通常表明日志损坏或程序实现错误。
//    - `Utf8`：把字节序列转换为 UTF-8 字符串失败（例如从 sled 读取到的值不是合法 UTF-8）。
//    - `Sled`：来自 sled 数据库的错误。
//    - `StringError`：通用的字符串消息错误，常用于把服务器端的业务错误传递给客户端或在测试中快速返回错误信息。
// 3. From 实现说明：
//    - 通过实现 `From<...>`，可以方便地用 `?` 操作符将底层错误自动转换为 `KvsError` 并向上传播，简化错误处理。
// 4. 对新手的建议：
//    - 在扩展库或增加新的错误场景时，优先考虑是否应该新增 `KvsError` 的变体或复用现有的 `StringError`。
//    - 使用 `failure::Fail` 能够提供 `Display` 与 `cause` 信息，但在新工程中也可考虑使用 `thiserror` 或 `anyhow` 等现代错误处理库。

impl From<io::Error> for KvsError {
    fn from(err: io::Error) -> KvsError {
        KvsError::Io(err)
    }
}

impl From<serde_json::Error> for KvsError {
    fn from(err: serde_json::Error) -> KvsError {
        KvsError::Serde(err)
    }
}

impl From<FromUtf8Error> for KvsError {
    fn from(err: FromUtf8Error) -> KvsError {
        KvsError::Utf8(err)
    }
}

impl From<sled::Error> for KvsError {
    fn from(err: sled::Error) -> KvsError {
        KvsError::Sled(err)
    }
}

/// Result type for kvs
pub type Result<T> = std::result::Result<T, KvsError>;
