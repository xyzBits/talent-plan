//! This module provides various thread pools. All thread pools should implement
//! the `ThreadPool` trait.

use crate::{thread_pool, Result};

mod naive;
mod rayon;
mod shared_queue;

pub use self::naive::NaiveThreadPool;
pub use self::rayon::RayonThreadPool;
pub use self::shared_queue::SharedQueueThreadPool;

/// The trait that all thread pools should implement.
/// 标准线程池的两个核心行为：初始化 new 和 派发任务 spawn
pub trait ThreadPool {
    /// Creates a new thread pool, immediately spawning the specified number of
    /// threads.
    ///
    /// Returns an error if any thread fails to spawn. All previously-spawned threads
    /// are terminated.
    /// threads 指定池子中要有几个线程
    /// 如果要开 4 个线程，前3个成功了，第4个失败了，不能返回半成品，要反前三个杀掉，清理干净，告诉用户创建失败
    fn new(threads: u32) -> Result<Self>
    // 创建可能成功，也可能失败，所以用 Result
    where
        // 只有具体大小已知的类型，才能调用 new
        Self: Sized; // 要求实现该 trait 的结构体大小必须是固定的，这在作为返回值是通常是必须 的

    /// Spawns a function into the thread pool.
    ///
    /// Spawning always succeeds, but if the function panics the threadpool continues
    /// to operate with the same number of threads &mdash; the thread count is not
    /// reduced nor is the thread pool destroyed, corrupted or invalidated.
    /// 将任务扔进池子中
    fn spawn<F>(&self, job: F)
    // F job 可以是任何类型，只要满足F 的约束
    where
        // 对任务的三个硬性要求
        F: FnOnce()
            // 任务必须是一个函数或闭包，()表示没有参数，也没有返回值，Once表示只会执行一次，线程拿到它，跑守，就销毁
            + Send
            // 并发安全的关键，任务在主线程创建，但要被转移send到子线程去执行，只有标记为Send的类型才能安全地跨越线程转移所有权
            //生命周期的关键，不是说闭包中的变量必须是全局静态，意思 是，闭包中捕获的变量不能包含任何非静态的引用、借用
            // 因为主线程派发任务后可能成上退出了，栈内存被销毁，闭包里如果还引用了主线程栈上的局部就是，子线程运行时
            // 就会出现非法访问，所以闭包必须拥有它所需要的数据的所有权 move 进去，或者数据本身是安全的
            + 'static;
}



// 详细说明（中文）：
// 1. 线程池的目的：线程创建与销毁开销大，尤其在高并发场景下频繁创建线程会极大影响性能和延迟。
//    线程池通过复用固定数量的线程来处理多个任务，从而将线程管理与任务调度解耦，降低上下文切换成本。
// 2. 为什么把 ThreadPool 抽象出来：
//    - 教学/工程上便于替换不同实现（比如简单的 shared-queue、基于 rayon 的线程池或其他实现）。
//    - 抽象让 `KvsServer` 与具体线程池实现解耦，便于测试和性能对比。
// 3. trait 设计要点：
//    - `new(threads)`：创建并立即 spawn 指定数量的线程；若线程创建失败，应返回 Err（这里使用 crate::Result）。
//    - `spawn(job)`：将一个 `FnOnce()` 任务提交到线程池，由线程池中的某个线程执行；任务需满足 `Send + 'static`。
// 4. 对 Rust 新手的建议：
//    - 线程池的关键是任务传递与线程生命周期管理（channel、队列、任务接收端的循环）。
//    - 注意线程间共享状态需要同步原语（Arc/Mutex、channel 等）；尽量把任务设计为无共享或通过消息传递来协调。
//    - 本仓库提供了三种简单实现以便理解不同权衡：`NaiveThreadPool`（最容易理解）、`SharedQueueThreadPool`（通用实现）、`RayonThreadPool`（基于成熟库的高性能实现）。
