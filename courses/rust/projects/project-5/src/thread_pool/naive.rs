use std::thread;

use super::ThreadPool;
use crate::Result;

/// 实际上这并不是一个线程池。它的实现方式是每次调用 `spawn` 方法时
/// 都创建一个新线程。
#[derive(Clone)]
pub struct NaiveThreadPool;

impl ThreadPool for NaiveThreadPool {
    fn new(_threads: u32) -> Result<Self> {
        Ok(NaiveThreadPool)
    }

    fn spawn<F>(&self, job: F)
    where
        F: FnOnce() + Send + 'static,
    {
        // 简单地为每个任务创建一个新线程，不进行重用
        thread::spawn(job);
    }
}
