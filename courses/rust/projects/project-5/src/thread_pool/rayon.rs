use super::ThreadPool;
use crate::{KvsError, Result};
use std::sync::Arc;

/// `rayon::ThreadPool` 的包装类。
#[derive(Clone)]
pub struct RayonThreadPool(Arc<rayon::ThreadPool>);

impl ThreadPool for RayonThreadPool {
    fn new(threads: u32) -> Result<Self> {
        // 使用 rayon 的 Builder 模式创建线程池
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(threads as usize)
            .build()
            .map_err(|e| KvsError::StringError(format!("{}", e)))?;
        Ok(RayonThreadPool(Arc::new(pool)))
    }

    fn spawn<F>(&self, job: F)
    where
        F: FnOnce() + Send + 'static,
    {
        // 派发任务到 rayon 线程池
        self.0.spawn(job)
    }
}
