use std::thread;

use super::ThreadPool;
use crate::Result;

use crossbeam::channel::{self, Receiver, Sender};

use log::{debug, error};

// Note for Rust training course: the thread pool is not implemented using
// `catch_unwind` because it would require the task to be `UnwindSafe`.

/// A thread pool using a shared queue inside.
///
/// If a spawned task panics, the old thread will be destroyed and a new one will be
/// created. It fails silently when any failure to create the thread at the OS level
/// is captured after the thread pool is created. So, the thread number in the pool
/// can decrease to zero, then spawning a task to the thread pool will panic.
/// 这段代码实现了一个 基于共享队列（Shared Queue）且具备“崩溃自愈”能力 的线程池。
///
///
pub struct SharedQueueThreadPool {
    // 发送端，专门发送 装箱的闭包 // 线程池本身不拥有线程，只是任务的发射器
    tx: Sender<Box<dyn FnOnce() + Send + 'static>>,
}

impl ThreadPool for SharedQueueThreadPool {
    fn new(threads: u32) -> Result<Self> {
        // 创建一个无界通道
        // 如果任务生产速度远已于消费速度，内存会爆炸
        let (tx, rx) = channel::unbounded::<Box<dyn FnOnce() + Send + 'static>>();

        for _ in 0..threads {
            // taskReceiver 包装
            let rx = TaskReceiver(rx.clone());
            thread::Builder::new().spawn(move || run_tasks(rx))?;
        }
        Ok(SharedQueueThreadPool { tx })
    }

    /// Spawns a function into the thread pool.
    ///
    /// # Panics
    ///
    /// Panics if the thread pool has no thread.
    fn spawn<F>(&self, job: F)
    where
        F: FnOnce() + Send + 'static,
    {
        self.tx
            .send(Box::new(job))
            .expect("The thread pool has no thread.");
    }
}

#[derive(Clone)]
struct TaskReceiver(Receiver<Box<dyn FnOnce() + Send + 'static>>);

impl Drop for TaskReceiver {
    fn drop(&mut self) {
        if thread::panicking() {
            let rx = self.clone();
            if let Err(e) = thread::Builder::new().spawn(move || run_tasks(rx)) {
                error!("Failed to spawn a thread: {}", e);
            }
        }
    }
}

fn run_tasks(rx: TaskReceiver) {
    loop {
        match rx.0.recv() {
            Ok(task) => {
                task();
            }
            Err(_) => debug!("Thread exits because the thread pool is destroyed."),
        }
    }
}

// 详细中文注释（补充）：
// 1. 实现说明：
//    - 使用 `crossbeam::channel::unbounded` 作为任务队列（无界队列），任务通过 `Sender` 提交。
//    - 每个线程持有一份 `Receiver` 的克隆（`rx.clone()`），线程在循环中 `recv()` 任务并执行。
// 2. panic 与线程恢复：
//    - 如果任务在执行时 panic，当前线程会因为 panic 终止；`TaskReceiver` 的 `Drop` 实现会检测到线程是在 panic 的上下文中退出，
//      尝试重新 spawn 一个线程来补充池中数量（此处的重建逻辑是“最佳努力”的，若重建失败会记录错误）。
//    - 这种策略能在一定程度上提升健壮性，但并非完全安全（如果线程反复 panic，可能导致频繁重建）。
// 3. 错误与边界条件：
//    - 使用无界 channel 可能导致在极端高负载下内存增长；可考虑使用有界队列并在满时返回错误或阻塞提交方。
//    - `spawn` 中的 `expect("The thread pool has no thread.")` 会在池中没有活跃线程时 panic，这里提示使用者配置线程数时应谨慎。
// 4. 对 Rust 新手的建议：
//    - 理解消息传递并发模型（channel）是实现线程池的核心之一；推荐先实现单生产者单消费者的简单案例再阅读这里的多消费者实现。
//    - 关注任务（闭包）在执行过程中访问共享数据时的同步（`Arc<Mutex<...>>` 或者尽量通过消息传递避免共享可变状态）。
// 5. 可改进之处（练手建议）：
//    - 将无界队列换为有界队列并实现背压；
//    - 增加健康检查和线程数量自适应（根据队列长度动态扩/缩容）；
//    - 在任务执行前后记录更多的监控信息（耗时、失败率），用于运维和调优。
