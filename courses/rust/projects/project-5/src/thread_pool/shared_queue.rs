use std::thread;

use super::ThreadPool;
use crate::Result;

use crossbeam::channel::{self, Receiver, Sender};

// 提示：此线程池没有使用 `catch_unwind` 实现，
// 因为这要求任务必须满足 `UnwindSafe` 约束。

/// 使用内部共享队列的线程池。
///
/// 如果派发的任务发生 panic，旧线程将被销毁并创建一个新线程。
/// 如果在线程池创建后，操作系统层面的线程创建失败，它会静默失败。
/// 因此，池中的线程数量可能会减少到零，此时向线程池派发任务将触发 panic。
#[derive(Clone)]
pub struct SharedQueueThreadPool {
    tx: Sender<Box<dyn FnOnce() + Send + 'static>>,
}

impl ThreadPool for SharedQueueThreadPool {
    fn new(threads: u32) -> Result<Self> {
        let (tx, rx) = channel::unbounded::<Box<dyn FnOnce() + Send + 'static>>();
        for _ in 0..threads {
            let rx = TaskReceiver(rx.clone());
            // 创建线程并运行任务提取循环
            thread::Builder::new().spawn(move || run_tasks(rx))?;
        }
        Ok(SharedQueueThreadPool { tx })
    }

    /// 将一个函数派发到线程池中。
    ///
    /// # Panics
    ///
    /// 如果线程池中没有任何线程（例如全部因创建失败而消失），则会发生 panic。
    fn spawn<F>(&self, job: F)
    where
        F: FnOnce() + Send + 'static,
    {
        self.tx
            .send(Box::new(job))
            .expect("The thread pool has no thread.");
    }
}

/// 包装接收端，用于实现 Drop trait 以便在 panic 时重启线程。
#[derive(Clone)]
struct TaskReceiver(Receiver<Box<dyn FnOnce() + Send + 'static>>);

impl Drop for TaskReceiver {
    fn drop(&mut self) {
        // 如果当前线程正在发生 panic，则尝试启动一个新线程来替代自己
        if thread::panicking() {
            let rx = self.clone();
            if let Err(e) = thread::Builder::new().spawn(move || run_tasks(rx)) {
                error!("Failed to spawn a thread: {}", e);
            }
        }
    }
}

/// 连续从接收端获取任务并执行。
fn run_tasks(rx: TaskReceiver) {
    loop {
        match rx.0.recv() {
            Ok(task) => {
                task();
            }
            Err(_) => {
                // 当 Sender 被丢弃（线程池销毁）时，接收端会返回错误，此时优雅退出
                debug!("Thread exits because the thread pool is destroyed.");
                break;
            }
        }
    }
}
