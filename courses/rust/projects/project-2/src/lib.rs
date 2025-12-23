#![deny(missing_docs)]
//! 一个简单的键值存储系统。

pub use error::{KvsError, Result};
pub use kv::KvStore;

/// 错误处理模块。
mod error;
/// 核心键值存储引擎模块。
mod kv;
