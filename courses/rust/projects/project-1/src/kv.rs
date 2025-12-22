#!/usr/bin/env rust
/// `kv.rs` 实现了一个简单的内存键值存储 `KvStore`。
///
/// 该实现使用 `HashMap<String, String>` 在内存中保存键值对，
/// 并提供 `new`, `set`, `get`, `remove` 四个基本操作。
/// 所有方法均带有中文注释，示例代码位于文档注释中。
use std::collections::HashMap;

/// `KvStore` 结构体，内部使用 `HashMap` 保存数据。
#[derive(Default)]
pub struct KvStore {
    /// 存储键值对的映射表。
    map: HashMap<String, String>,
}

impl KvStore {
    /// 创建一个空的 `KvStore` 实例。
    pub fn new() -> KvStore {
        KvStore { map: HashMap::new() }
    }

    /// 将 `key` 与 `value` 关联，若键已存在则覆盖旧值。
    pub fn set(&mut self, key: String, value: String) {
        self.map.insert(key, value);
    }

    /// 根据 `key` 查询对应的值，若不存在返回 `None`。
    pub fn get(&self, key: String) -> Option<String> {
        self.map.get(&key).cloned()
    }

    /// 删除指定的 `key`，若键不存在则不做任何操作。
    pub fn remove(&mut self, key: String) {
        self.map.remove(&key);
    }
}
