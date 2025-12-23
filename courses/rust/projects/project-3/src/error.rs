use failure::Fail;
use std::io;
use std::string::FromUtf8Error;

/// kvs 的错误类型
#[derive(Fail, Debug)]
pub enum KvsError {
    /// IO 错误
    #[fail(display = "IO error: {}", _0)]
    Io(#[cause] io::Error),
    /// 序列化或反序列化错误
    #[fail(display = "serde_json error: {}", _0)]
    Serde(#[cause] serde_json::Error),
    /// 移除不存在的键时触发的错误
    #[fail(display = "Key not found")]
    KeyNotFound,
    /// 非预期的命令类型错误
    /// 这通常表示日志损坏或程序存在逻辑缺陷
    #[fail(display = "Unexpected command type")]
    UnexpectedCommandType,
    /// 键或值不是合法的 UTF-8 序列
    #[fail(display = "UTF-8 error: {}", _0)]
    Utf8(#[cause] FromUtf8Error),
    /// Sled 存储引擎返回的错误
    #[fail(display = "sled error: {}", _0)]
    Sled(#[cause] sled::Error),
    /// 包含自定义字符串消息的错误
    #[fail(display = "{}", _0)]
    StringError(String),
}

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

/// kvs 的 Result 类型，简化了 KvsError 的返回
pub type Result<T> = std::result::Result<T, KvsError>;
