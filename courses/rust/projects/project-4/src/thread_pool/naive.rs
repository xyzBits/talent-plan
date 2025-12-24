use std::thread;

use super::ThreadPool;
use crate::Result;

/// It is actually not a thread pool. It spawns a new thread every time
/// the `spawn` method is called.
pub struct NaiveThreadPool;

impl ThreadPool for NaiveThreadPool {
    fn new(_threads: u32) -> Result<Self> {
        Ok(NaiveThreadPool)
    }

    fn spawn<F>(&self, job: F)
    where
        F: FnOnce() + Send + 'static,
    {
        thread::spawn(job);
    }
}

// 详细中文注释（补充，不删除已有注释）：
// 1. 本实现名为 "NaiveThreadPool"，但严格来说并不是一个真正的线程池：
//    每次调用 `spawn` 都会创建一个新的操作系统线程并立即运行任务。
// 2. 适用场景与限制：
//    - 适用于演示、测试或非常低并发的场景；实现非常简单，便于理解线程创建流程。
//    - 在高并发或长期运行的服务中会导致大量线程被创建，消耗内存并引发上下文切换，可能导致性能下降。
// 3. 对 Rust 新手的建议：
//    - 了解 `thread::spawn` 的返回值是 `JoinHandle`，如果不 `join`，线程会在后台运行；此实现没有管理线程句柄。
//    - 若任务可能 panic 且你想捕获 panic，请阅读 `std::panic::catch_unwind`，但那会对任务边界和类型安全有额外要求（UnwindSafe）。
// 4. 扩展建议：
//    - 想要实现真正的线程池，需要保持一组固定线程并提供一个任务队列（如 channel），线程从队列中取任务并执行。
//    - 可参考本仓库的 `SharedQueueThreadPool` 作为一个更现实的实现。
