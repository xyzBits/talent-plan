pub use self::kvs::KvStore;
pub use self::sled::SledKvsEngine;
use crate::Result;

mod kvs;
mod sled;

/// Trait for a key value storage engine.
pub trait KvsEngine: Clone + Send + 'static {
    /// Sets the value of a string key to a string.
    ///
    /// If the key already exists, the previous value will be overwritten.
    fn set(&self, key: String, value: String) -> Result<()>;

    /// Gets the string value of a given string key.
    ///
    /// Returns `None` if the given key does not exist.
    fn get(&self, key: String) -> Result<Option<String>>;

    /// Removes a given key.
    ///
    /// # Errors
    ///
    /// It returns `KvsError::KeyNotFound` if the given key is not found.
    fn remove(&self, key: String) -> Result<()>;
}

// 详细中文注释（补充）：
// 1. trait 设计说明：
//    - `KvsEngine` 将存储引擎抽象为一个 trait，使得服务器和客户端逻辑可以与具体实现解耦，
//      从而可以在不同项目或不同运行时选择不同的底层实现（例如 file-based `KvStore` 或基于 `sled` 的实现）。
// 2. 方法接收者使用 `&self` 而不是 `&mut self`：
//    - 选择 `&self` 的原因是希望 `KvsEngine` 的实现能够以内部可变性（interior mutability）或线程安全的共享方式实现并发访问，
//      例如在实现中使用 `Arc<Mutex<...>>`、`Arc<RwLock<...>>` 或无锁数据结构 (`SkipMap`) 等。
//    - 这也使得 `KvsEngine` 更容易实现 `Clone`，因为通常只是克隆 `Arc` 的句柄而非复制底层数据。
// 3. trait bounds（`Clone + Send + 'static`）：
//    - `Clone`：服务器在为每个连接创建任务时需要克隆引擎句柄（通常为轻量的 `Arc` 克隆）。
//    - `Send` 与 `'static`：确保引擎可以安全地跨线程传递并在后台任务中持有（线程池/任务会要求此约束）。
// 4. 对新手的建议：
//    - 在实现 `KvsEngine` 时优先使用 `Arc` + 内部可变性或线程安全的数据结构，这样服务器可以在多线程中高效复用引擎实例。
//    - 如果你的实现需要可变借用（`&mut self`），需要注意如何在服务器并发场景中保证安全（通常会使用 mutex 或把写操作串行化）。
