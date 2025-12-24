use super::ThreadPool;
use crate::{KvsError, Result};

/// Wrapper of rayon::ThreadPool
pub struct RayonThreadPool(rayon::ThreadPool);

impl ThreadPool for RayonThreadPool {
    fn new(threads: u32) -> Result<Self> {
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(threads as usize)
            .build()
            .map_err(|e| KvsError::StringError(format!("{}", e)))?;
        Ok(RayonThreadPool(pool))
    }

    fn spawn<F>(&self, job: F)
    where
        F: FnOnce() + Send + 'static,
    {
        self.0.spawn(job)
    }
}

// 详细中文注释（补充）：
// 1. 说明：`RayonThreadPool` 是对第三方库 `rayon` 的封装。`rayon` 提供了高性能的工作窃取线程池，
//    适合 CPU 密集型任务或需要并行迭代的场景。
// 2. 与 SharedQueue 的差别：
//    - `SharedQueueThreadPool` 是一个显式的任务队列 + 固定线程实现，适合简单场景并便于学习。
//    - `rayon` 使用工作窃取（work-stealing）算法，线程会在本地队列执行任务并在空闲时从其他线程窃取任务，
//      在某些并行计算模式下能带来更优的负载均衡与性能。
// 3. 对 Rust 新手的建议：
//    - `rayon` 更适用于数据并行（如并行集合处理），对短小且大量的独立任务也有良好表现。
//    - 注意 `rayon::ThreadPool::spawn` 语义与你直接 `std::thread::spawn` 的差别：`rayon` 管理的是逻辑任务队列，
//      task 的调度由 `rayon` 内部策略决定，不一定会对应到具体的 OS 线程数量。
// 4. 错误处理：构建线程池失败会被映射为 `KvsError::StringError`（此错误类型在 crate 中定义），调用者应当处理返回的 `Err`。
