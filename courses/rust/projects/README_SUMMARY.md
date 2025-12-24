# 五个项目进化概述（摘要）

本汇总描述 workspace 中 `project-1` 到 `project-5` 的递进关系、每一阶段的改进与优化，并给出必要代码对比以突出设计演进。

**快速导航**
- **Project 1**: 极简内存实现，见 [project-1/src/kv.rs](project-1/src/kv.rs)
- **Project 2**: 持久化日志（log-structured）、索引与压缩，见 [project-2/src/kv.rs](project-2/src/kv.rs)
- **Project 3**: 引入引擎抽象与网络服务，见 [project-3/src/engines/mod.rs](project-3/src/engines/mod.rs) 与 [project-3/src/server.rs](project-3/src/server.rs)
- **Project 4**: 添加线程池抽象并在服务器中使用，见 [project-4/src/thread_pool/mod.rs](project-4/src/thread_pool/mod.rs) 与 [project-4/src/server.rs](project-4/src/server.rs)
- **Project 5**: 异步/并发化（Tokio + futures）、更复杂的并发 KvStore 实现与混合线程池 + futures，见 [project-5/src/engines/kvs.rs](project-5/src/engines/kvs.rs) 与 [project-5/src/server.rs](project-5/src/server.rs)

**整体递进要点**
- 从单进程内存（HashMap）→ 持久化日志 + 索引 → 引擎抽象（可替换存储）→ 并发处理（线程池）→ 异步/更高并发架构。
- 每一步都在可用性、持久性、并发性或模块化（可替换性）上做出改进。

**逐项目要点与代码对比**

**Project 1 — 基础（内存）**
- 实现：`KvStore` 基于 `HashMap<String,String>`，提供 `new/set/get/remove`。短小、无持久化、易理解。
- 关键示例（摘录）：
```text
pub struct KvStore { map: HashMap<String,String> }
impl KvStore {
  pub fn new() -> KvStore { ... }
  pub fn set(&mut self, key:String, value:String) { self.map.insert(key,value); }
  pub fn get(&self, key:String) -> Option<String> { self.map.get(&key).cloned() }
}
```

**Project 2 — 持久化日志与压缩（核心存储）**
- 改进：引入日志文件（generation `.log`）、内存索引（`BTreeMap`）、按需压缩（compaction）、错误类型 `KvsError`。
- 设计要点：顺序追加写（sequential write）、内存索引映射到磁盘位置、重启时重放日志恢复状态。
- 与 Project 1 的对比（核心变化）：
  - Project 1 的 `set` 直接写入内存；Project 2 的 `set` 先序列化并追加到日志，再更新内存索引：
```diff
- // project-1: 内存写
- self.map.insert(key, value);
+ // project-2: 日志追加 + flush
+ serde_json::to_writer(&mut self.writer, &cmd)?;
+ self.writer.flush()?;
+ self.index.insert(key, (self.current_gen, pos..self.writer.pos).into());
```

**Project 3 — 引擎抽象与网络服务**
- 改进：把存储实现抽象为 `KvsEngine` trait，使得底层实现可替换（内置 file-based `KvStore` 与 `SledKvsEngine`），并加入简单的 TCP 同步服务器/客户端。
- 关键接口（摘录）：
```text
pub trait KvsEngine {
  fn set(&mut self, key:String, value:String)->Result<()>;
  fn get(&mut self, key:String)->Result<Option<String>>;
  fn remove(&mut self, key:String)->Result<()>;
}
```
- 服务器变化：从命令行工具（project-1）升级为网络服务，单线程或同步方式处理请求（见 [project-3/src/server.rs](project-3/src/server.rs)）。

**Project 4 — 线程池（并发服务器）**
- 改进：引入 `ThreadPool` trait 与多种实现（`NaiveThreadPool`、`SharedQueueThreadPool`、`RayonThreadPool`），并在服务器端使用线程池把每个连接提交为任务，从而并发处理客户端请求。
- 关键变化（server run 的对比）：
```diff
- // project-3: 同步处理每个连接
- for stream in listener.incoming() { serve(stream)?; }
+ // project-4: 使用线程池并行处理
+ for stream in listener.incoming() {
+   let engine = self.engine.clone();
+   self.pool.spawn(move || match stream { Ok(s)=>serve(engine,s), Err(e)=>... });
+ }
```

**Project 5 — 异步与高并发（Tokio + futures）**
- 改进：全面采用异步 IO（Tokio、tokio-serde-json、LengthDelimitedCodec），KvStore 的 API 返回 `Future`，内部进一步优化并发数据结构（`SkipMap`、读者池 `ArrayQueue`、写/读分离、`Arc/Mutex` 封装）、兼顾 Windows 的文件删除差异处理。
- 关键变化：
  - 服务器变为基于 `tokio` 的异步执行，使用 framed + JSON 的协议（提高吞吐与兼容性）。见 [project-5/src/server.rs](project-5/src/server.rs)。
  - 存储内部：用 `SkipMap` 代替 `BTreeMap`（无锁/并发友好），引入 `KvStoreReader`/`KvStoreWriter`、reader 池、以及把写入任务提交到线程池並通过 oneshot 通道返回結果，`set/get/remove` 返回 `Future` 对象（見 [project-5/src/engines/kvs.rs](project-5/src/engines/kvs.rs)）。

示例（Project 5 中 `set` 返回 Future 的简化示意）：
```text
fn set(&self, key, value) -> Box<dyn Future<Item=(), Error=KvsError> + Send> {
  let writer = self.writer.clone();
  let (tx,rx) = oneshot::channel();
  self.thread_pool.spawn(move || { let res = writer.lock().unwrap().set(key,value); tx.send(res); });
  Box::new(rx.flatten())
}
```

**总结：每一步的收益（按维度）**
- 可用性：Project1 → Project2（持久化保证），Project3（网络化），Project5（异步服务更高可用）。
- 性能/吞吐：顺序写日志（Project2）提升写入性能；Project4 的线程池与 Project5 的 tokio 异步均提升并发吞吐；Project5 的读者池与 SkipMap 减少读写竞争。
- 模块化/可替换性：Project3 引入 `KvsEngine`，使得 `sled` 或 file-based 引擎可互换；Project4/5 在此之上引入不同并发策略。

**如何快速查看代码与运行**
- 浏览实现：参考项目目录下相应文件，例如 [project-2/src/kv.rs](project-2/src/kv.rs) 与 [project-5/src/engines/kvs.rs](project-5/src/engines/kvs.rs)。
- 运行示例（以某个 project 为例）:
```bash
cd project-5
cargo run --bin kvs-server -- --addr 127.0.0.1:4000
# 另开终端运行 client
cargo run --bin kvs-client -- --addr 127.0.0.1:4000 set key value
```

-----
如需我把此 README 转成某个项目目录下的 README.md（或把内容拆成多个文档：架构/性能/代码对比），我可以继续分拆并加入更详细的代码对比片段与行号引用。
