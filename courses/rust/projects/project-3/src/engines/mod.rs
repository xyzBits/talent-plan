//! This module provides various key value storage engines.

use crate::Result;

/// 定义存储引擎的通用接口。
pub trait KvsEngine {
    /// 设置给定字符串键的值为字符串。
    ///
    /// 如果该键已存在，则覆盖旧值。
    fn set(&mut self, key: String, value: String) -> Result<()>;

    /// 获取给定字符串键的字符串值。
    ///
    /// 如果键不存在，则返回 `None`。
    fn get(&mut self, key: String) -> Result<Option<String>>;

    /// 删除指定的键。
    ///
    /// # 错误
    ///
    /// 如果键不存在，则返回 `KvsError::KeyNotFound`。
    fn remove(&mut self, key: String) -> Result<()>;
}

mod kvs;
mod sled;

pub use self::kvs::KvStore;
pub use self::sled::SledKvsEngine;
