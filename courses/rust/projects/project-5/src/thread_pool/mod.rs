//! 此模块提供多种线程池实现。所有的线程池都应实现
//! `ThreadPool` trait。

use crate::Result;

mod naive;
mod rayon;
mod shared_queue;

pub use self::naive::NaiveThreadPool;
pub use self::rayon::RayonThreadPool;
pub use self::shared_queue::SharedQueueThreadPool;

/// 所有线程池必须实现的 trait。
pub trait ThreadPool: Clone + Send + 'static {
    /// 创建一个新的线程池，并立即启动指定数量的线程。
    ///
    /// 如果任何线程启动失败，则返回错误。所有先前已启动的线程都会被终止。
    fn new(threads: u32) -> Result<Self>
    where
        Self: Sized;

    /// 将一个函数派发到线程池中执行。
    ///
    /// 派发操作总是成功的。如果派发的函数内部发生 panic，
    /// 线程池应保持运行，且线程数量不应减少。
    fn spawn<F>(&self, job: F)
    where
        F: FnOnce() + Send + 'static;
}
