use super::KvsEngine;
use crate::{KvsError, Result};
use sled::{Db, Tree};

/// Wrapper of `sled::Db`
#[derive(Clone)]
pub struct SledKvsEngine(Db);

// 详细中文注释（补充）：
// 1. 目的：`SledKvsEngine` 是对 `sled::Db` 的轻量封装，使其实现 `KvsEngine` 接口，从而可以在同一套服务器逻辑中
//    作为替代的持久化后端使用。相比自实现的日志结构，`sled` 提供了成熟的键值存储语义以及更复杂的并发控制。
// 2. 设计要点：
//    - `SledKvsEngine` 包含 `sled::Db`，实现了 `set/get/remove`，并且所有方法返回 `Result<T, KvsError>`，
//      通过 `impl From<sled::Error> for KvsError` 把 `sled` 的错误映射为通用错误类型。
//    - `set` 会调用 `tree.insert` 并 `flush`，以确保数据落盘；`get` 会把 `sled` 返回的字节向量尝试转换为 `String`（UTF-8），
//      若转换失败则返回 `KvsError::Utf8`（上层会处理该错误）。
// 3. 对新手的建议：
//    - 使用第三方存储引擎可以节省实现细节，但需要关注数据模型与 API 语义差异（例如 `sled` 的原子性、事务支持等）。
//    - 在高并发场景下，`sled` 的表现通常优于手写单文件日志实现，因为它针对并发与磁盘访问做了许多优化。

impl SledKvsEngine {
    /// Creates a `SledKvsEngine` from `sled::Db`.
    pub fn new(db: Db) -> Self {
        SledKvsEngine(db)
    }
}

impl KvsEngine for SledKvsEngine {
    fn set(&self, key: String, value: String) -> Result<()> {
        let tree: &Tree = &self.0;
        tree.insert(key, value.into_bytes()).map(|_| ())?;
        tree.flush()?;
        Ok(())
    }

    fn get(&self, key: String) -> Result<Option<String>> {
        let tree: &Tree = &self.0;
        Ok(tree
            .get(key)?
            .map(|i_vec| AsRef::<[u8]>::as_ref(&i_vec).to_vec())
            .map(String::from_utf8)
            .transpose()?)
    }

    fn remove(&self, key: String) -> Result<()> {
        let tree: &Tree = &self.0;
        tree.remove(key)?.ok_or(KvsError::KeyNotFound)?;
        tree.flush()?;
        Ok(())
    }
}
