#![deny(missing_docs)]
//! 一个简单的键值存储库，支持异步网络通信。

#[macro_use]
extern crate log;

// 重新导出核心组件，方便外部使用
pub use client::KvsClient;
pub use engines::{KvStore, KvsEngine, SledKvsEngine};
pub use error::{KvsError, Result};
pub use server::KvsServer;

mod client;
mod common;
mod engines;
mod error;
mod server;
pub mod thread_pool;
