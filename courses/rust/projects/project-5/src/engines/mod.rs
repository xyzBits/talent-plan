pub use self::kvs::KvStore;
pub use self::sled::SledKvsEngine;
use crate::KvsError;

use tokio::prelude::Future;

mod kvs;
mod sled;

/// 键值存储引擎接口。
/// 所有的引擎方法都返回一个 Future，允许异步处理。
/// 引擎必须实现 Clone + Send，以便在多个线程间共享。
pub trait KvsEngine: Clone + Send + 'static {
    /// 设置键的值。
    /// 如果键已存在，其先前的值将被覆盖。
    fn set(&self, key: String, value: String) -> Box<dyn Future<Item = (), Error = KvsError> + Send>;

    /// 获取给定键的值。
    /// 如果键不存在，返回 `None`。
    fn get(&self, key: String) -> Box<dyn Future<Item = Option<String>, Error = KvsError> + Send>;

    /// 移除给定的键。
    ///
    /// # 错误
    ///
    /// 如果键不存在，返回 `KvsError::KeyNotFound`。
    fn remove(&self, key: String) -> Box<dyn Future<Item = (), Error = KvsError> + Send>;
}
