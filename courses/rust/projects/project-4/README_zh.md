# PNA Rust 项目 4：并发与并行（中文翻译）

**任务**：实现一个多线程、持久化的键值存储服务器与客户端，使用自定义协议进行同步网络通信。

**目标**：

- 编写一个简单的线程池
- 使用通道（channels）进行线程间通信
- 用锁共享数据结构
- 在读操作中尽可能避免使用锁
- 对单线程与多线程实现进行基准测试对比

**主题**：线程池、通道、锁、无锁数据结构、原子操作、参数化基准测试。

- [简介](#introduction)
- [项目规范](#project-spec)
- [项目设置](#project-setup)
- [背景：阻塞与多线程](#background-blocking-and-multithreading)
- [第 1 部分：多线程](#part-1-multithreading)
- [第 2 部分：创建共享的 `KvsEngine`](#part-2-creating-a-shared-kvsengine)
- [第 3 部分：为 `KvServer` 添加多线程支持](#part-3-adding-multithreading-to-kvserver)
- [第 4 部分：实现真正的线程池](#part-4-creating-a-real-thread-pool)
	- [如何构建线程池？](#so-how-do-you-build-a-thread-pool)
- [第 5 部分：抽象的线程池](#part-5-abstracted-thread-pools)
- [第 6 部分：评估你的线程池](#part-6-evaluating-your-thread-pool)
	- [首先两个基准测试](#ok-now-to-the-first-two-benchmarks)
- [第 7 部分：评估其它线程池与引擎](#part-7-evaluating-other-thread-pools-and-engines)
	- [扩展 1：比较函数](#extension-1-comparing-functions)
	- [背景：锁的局限](#background-the-limits-of-locks)
- [第 8 部分：无锁读操作](#part-8-lock-free-readers)
	- [解释示例数据结构](#explaining-our-example-data-structure)
	- [打破锁的策略](#strategies-for-breaking-up-locks)
		- [理解并维护顺序一致性](#understand-and-maintain-sequential-consistency)
		- [识别不可变值](#identify-immutable-values)
		- [复制值而不是共享](#duplicate-values-instead-of-sharing)
		- [按角色拆分数据结构](#break-up-data-structures-by-role)
		- [使用专用并发数据结构](#use-specialized-concurrent-data-structures)
		- [把清理推迟到以后](#postpone-cleanup-until-later)
		- [用原子类型共享标志和计数器](#share-flags-and-counters-with-atomics)
	- [实现无锁读操作](#implement-lock-free-readers)

## 简介

在本项目中，你将实现一个简单的键值服务器和客户端，二者通过自定义协议通信。服务器使用同步网络（blocking I/O），并通过逐步改进的并发实现来响应多个请求。内存索引会演进为一个被所有线程共享的并发数据结构，压缩（compaction）将在后台线程中进行，以降低单个请求的延迟。

## 项目规范

该 cargo 工程名为 `kvs`，构建出两个可执行文件：命令行客户端 `kvs-client` 与服务器 `kvs-server`，两者都调用库 `kvs`。客户端与服务器通过自定义协议通信。

命令行接口与上一个项目一致，但本次主要差别在于并发实现，会在下面逐步介绍。

库接口与之前相似，但有两点不同：

1. 本次 `KvsEngine`、`KvStore` 等的方法改为接收 `&self`（而非 `&mut self`），并要求实现 `Clone`，这是并发数据结构常见的模式。
2. 新增一个 `ThreadPool` trait，包含如下方法：

- `ThreadPool::new(threads: u32) -> Result<ThreadPool>`：创建并立刻启动指定数量线程的线程池；若任一线程启动失败则返回错误并终止已启动线程。
- `ThreadPool::spawn<F>(&self, job: F) where F: FnOnce() + Send + 'static`：向线程池提交一个任务。提交总是成功的；若任务 panic，线程池应继续以相同线程数运行（不应被破坏或减少线程数）。

在本项目结束时，你将实现多个该 trait 的实现，并对比它们的性能。

本项目不应需要修改客户端代码。

## 项目设置

延续上个项目的工作，删除旧的 `tests` 目录并复制本项目的 `tests` 目录到合适位置。项目应包含一个名为 `kvs` 的库与两个可执行文件：`kvs-server` 和 `kvs-client`。

在 `Cargo.toml` 中需要如下 dev-dependencies：

```toml
[dev-dependencies]
assert_cmd = "0.11"
criterion = "0.2.11"
crossbeam-utils = "0.6.5"
predicates = "1.0.0"
rand = "0.6.5"
tempfile = "3.0.7"
walkdir = "2.2.7"
panic-control = "0.1.4"
```

像之前一样，先添加必要的定义以便测试套件能构建。

## 背景：阻塞与多线程

到目前为止，你的服务在单线程上顺序处理所有读写请求（例如 `get` 与 `set`），也就是说所有请求都是被序列化执行的。为了在某些请求阻塞（例如等待磁盘 I/O）时继续让 CPU 工作，我们可以把请求分发到多线程，从而在多核机器上实现并发甚至并行处理。

本项目的重点就是把请求并行处理，从而提升吞吐量并降低阻塞对整体性能的影响。

## 第 1 部分：多线程

最简单的并发尝试是为每个到来的连接 spawn 一个线程，在该线程中处理该连接的请求，处理完成后线程退出。实现一个名为 `NaiveThreadPool` 的 `ThreadPool`，其 `spawn` 方法为每个任务新建线程（注意这其实并不是重用线程的真正线程池，但须实现同一 trait 以便后续比较）。

完成后将 `NaiveThreadPool` 集成进 `KvServer`，观察延迟与吞吐量的变化。

测试用例：`thread_pool::naive_thread_pool_*`

## 第 2 部分：创建共享的 `KvsEngine`

在把 `NaiveThreadPool` 集成到 `KvServer` 之前，需要调整 `KvsEngine` trait 与 `KvStore` 的实现。本次 `KvsEngine` 的方法接受 `&self`（而非 `&mut self`），并要求实现 `Clone`、`Send` 和 `'static`，因此实现时通常会把实际数据放到堆上并用线程安全的共享指针（例如 `Arc`）与同步原语包裹。

示例 trait：

```rust
pub trait KvsEngine: Clone + Send + 'static {
		fn set(&self, key: String, value: String) -> Result<()>;
		fn get(&self, key: String) -> Result<Option<String>>;
		fn remove(&self, key: String) -> Result<()>;
}
```

这个设计把 engine 当作一个可在线程间克隆的“句柄”，底层共享状态放在堆上并由合适的同步机制保护。

在此阶段，将单线程 `kvs-server` 改造为使用可共享的 `KvsEngine`，使其可以在后续被多个线程共享。

测试用例：`kv_store::concurrent_*`

## 第 3 部分：为 `KvServer` 添加多线程支持

回顾架构：`KvServer` 监听 TCP，反序列化请求，调用 `KvsEngine` 接口处理并返回响应；`KvServer` 不关心引擎的内部实现。

现在把循环内的工作（读取请求、执行 engine 操作、写回响应）放到 `NaiveThreadPool` 中执行，从而让监听线程更快恢复接收更多连接，提高吞吐量。

## 第 4 部分：实现真正的线程池

为提高性能，你需要实现一个真正的线程池（例如 `SharedQueueThreadPool`），重用固定数量的线程并通过一个共享队列分发任务。线程复用可以减少频繁创建销毁线程的开销（如栈分配、syscall 等）。

如何实现线程池的关键点：

1. 选择用于分发任务的数据结构（通常为队列），考虑生产者/消费者模型；
2. 处理任务 panic 的策略（让线程死亡并重启，或捕获 panic 并继续运行）；
3. 正确优雅的关闭线程池（确保 `ThreadPool` drop 时能停止所有线程）。

常用工具：`thread::spawn`、`thread::panicking`、`catch_unwind`、`mpsc`、`Mutex`、crossbeam 的 MPMC 通道、`JoinHandle` 等。

示例消息枚举：

```rust
enum ThreadPoolMessage {
		RunJob(Box<dyn FnOnce() + Send + 'static>),
		Shutdown,
}
```

用 `num_cpus` crate 创建初始线程数（例如每个 CPU 一个线程）作为起点。

测试用例：`shared_queue_thread_pool_*`

## 第 5 部分：抽象的线程池

像之前对 `KvsEngine` 的抽象一样，为 `KvServer` 添加第二个类型参数表示 `ThreadPool` 的实现，并在构造时传入线程池。此后再实现另一个 `ThreadPool`（例如基于 `rayon` 的 `RayonThreadPool`）以对比性能。

## 第 6 部分：评估你的线程池

编写六个基准（criterion）：

1. 写密集型：`SharedQueueThreadPool`，不同线程数；
2. 读密集型：`SharedQueueThreadPool`，不同线程数；
3-4. 同上两项但使用 `RayonThreadPool`；
5-6. 使用 `RayonThreadPool` 并切换为 `SledKvsEngine` 的读/写测试。

使用参数化基准（benchmarking with inputs）对比不同线程数（例如 1、2、4、直到 2x CPU 个数），分析线程数对吞吐量的影响。注意：你的 `KvsClient` 可能是阻塞的，因此在基准中通常需要为每个请求使用独立线程以饱和服务器，或提前构造可复用的客户端线程池来降低测量干扰。

示例基准设置（criterion）说明如何把耗时测量的主体放入 `b.iter(...)` 中，而将耗时较大的 setup 放到循环外。

运行基准：

```bash
cargo bench
```

查看 criterion 生成的图表，观察不同线程数时的趋势，并分析结果。

### 首先两个基准测试

写密集型：在 setup 中创建 `KvServer<KvStore, SharedQueueThreadPool>`，线程池线程数为基准参数。发起 1000 个固定（但在每次迭代中相同）的写请求并断言成功，然后在所有请求完成后结束该次迭代。

读密集型：在 setup 中创建服务器（同上）并初始化 1000 个相同长度的键，然后在基准循环中从客户端并发发起 1000 个读请求并断言结果正确。

注意：阻塞客户端会使得需要大量客户端线程以饱和服务器，此时可借助 `SharedQueueThreadPool` 做好线程重用与测量准备。

## 第 7 部分：评估其它线程池与引擎

将上述两组基准复制并把 `SharedQueueThreadPool` 替换为 `RayonThreadPool`；再把引擎替换为 `SledKvsEngine`，比较结果并分析原因。

可以参考 `rayon` 和 `sled` 源码以更好理解其性能特性。

### 扩展 1：比较函数

Criterion 支持比较多个实现。阅读其文档的“comparing functions”部分以生成漂亮的对比图表。

### 背景：锁的局限

把整个引擎的状态包在 `Arc<Mutex<T>>` 中是简单可靠的方案，但在并发下会成为性能瓶颈，因为 `Mutex` 同时序列化读与写访问。改进方式之一是用 `RwLock`，允许并发读或单写，但写操作仍然会阻塞读。

更高级的目标是消除读路径上的锁（无锁读），使得读操作即使在写发生时也能继续进行，以获得更好并行度。

## 第 8 部分：无锁读操作

本部分挑战你实现读操作不加锁（即使存在并发写）。写仍可同期阻塞其它写，但读必须尽量不被写阻塞。实现这一点需要逐字段地分析共享状态并为不同字段选择合适的并发策略（复制、专用并发数据结构、原子变量、延迟清理等）。

示例：原始的单线程 `KvStore` 结构与把它改为 `Arc<Mutex<SharedKvStore>>` 的方法都展示了从简单到粗粒度锁的过程；进一步可用 `RwLock` 改善读并发，但最佳做法是为每个成员选择合适的并发方案，从而实现读写并行。

在没有大锁的前提下，你需要保证诸如索引中的日志指针总是指向有效日志、`uncompacted` 的统计合理等不变量，同时实现安全的 compaction。

### 关键策略概述

- 理解并维护顺序一致性（happens-before 关系）；
- 识别并利用不可变值（可安全共享，例如 `PathBuf`）；
- 在合适场景下复制值替代共享；
- 按角色（读/写/压缩器）拆分数据结构；
- 使用专用并发数据结构（如并发 map/skiplist）；
- 把清理（资源回收）推迟到合适时机（借助 epoch、引用计数或专门回收策略）；
- 使用原子类型共享标志与计数器，以减少锁争用。

理论与实践相结合，目标是在保证一致性的同时尽量降低锁竞争，从而提升并发性能。

### 实现无锁读操作

任务：修改 `KvStore` 以支持读写并发，读操作在大多数情况下不加锁。

祝编码愉快，完成后好好休息一下。

