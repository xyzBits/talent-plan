//! 这是一个示例 Crate。
//!
//! 这里写关于你这个项目的简要介绍，比如它是做什么的。
#![deny(missing_docs)]
/// 简单的键值存储库入口模块。
///
/// 本模块公开 `KvStore` 类型，供外部使用，并包含内部实现模块 `kv`。
pub use kv::KvStore;

/// 内部实现模块，定义 `KvStore` 的具体结构与方法。
mod kv;
