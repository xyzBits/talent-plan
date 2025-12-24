use crate::common::{GetResponse, RemoveResponse, Request, SetResponse};
use crate::thread_pool::ThreadPool;
use crate::{KvsEngine, Result};
use log::{debug, error};
use serde_json::Deserializer;
use std::io::{BufReader, BufWriter, Write};
use std::net::{TcpListener, TcpStream, ToSocketAddrs};

/// The server of a key value store.
pub struct KvsServer<E: KvsEngine, P: ThreadPool> {
    engine: E,
    pool: P,
}

impl<E: KvsEngine, P: ThreadPool> KvsServer<E, P> {
    /// Create a `KvsServer` with a given storage engine.
    pub fn new(engine: E, pool: P) -> Self {
        KvsServer { engine, pool }
    }

    /// Run the server listening on the given address
    pub fn run<A: ToSocketAddrs>(self, addr: A) -> Result<()> {
        let listener = TcpListener::bind(addr)?;
        for stream in listener.incoming() {
            let engine = self.engine.clone();
            self.pool.spawn(move || match stream {
                Ok(stream) => {
                    if let Err(e) = serve(engine, stream) {
                        error!("Error on serving client: {}", e);
                    }
                }
                Err(e) => error!("Connection failed: {}", e),
            })
        }
        Ok(())
    }
}

fn serve<E: KvsEngine>(engine: E, tcp: TcpStream) -> Result<()> {
    let peer_addr = tcp.peer_addr()?;
    let reader = BufReader::new(&tcp);
    let mut writer = BufWriter::new(&tcp);
    let req_reader = Deserializer::from_reader(reader).into_iter::<Request>();

    macro_rules! send_resp {
        ($resp:expr) => {{
            let resp = $resp;
            serde_json::to_writer(&mut writer, &resp)?;
            writer.flush()?;
            debug!("Response sent to {}: {:?}", peer_addr, resp);
        };};
    }

    for req in req_reader {
        let req = req?;
        debug!("Receive request from {}: {:?}", peer_addr, req);
        match req {
            Request::Get { key } => send_resp!(match engine.get(key) {
                Ok(value) => GetResponse::Ok(value),
                Err(e) => GetResponse::Err(format!("{}", e)),
            }),
            Request::Set { key, value } => send_resp!(match engine.set(key, value) {
                Ok(_) => SetResponse::Ok(()),
                Err(e) => SetResponse::Err(format!("{}", e)),
            }),
            Request::Remove { key } => send_resp!(match engine.remove(key) {
                Ok(_) => RemoveResponse::Ok(()),
                Err(e) => RemoveResponse::Err(format!("{}", e)),
            }),
        };
    }
    Ok(())
}

// 详细中文注释（补充，不删除已有注释）：
// 1. 设计概述：
//    - `KvsServer` 是处理网络请求的入口，使用泛型 `E: KvsEngine` 表示存储引擎，`P: ThreadPool` 表示并发任务执行策略。
//    - 这样设计使得存储实现和并发模型可替换（例如可用 `SledKvsEngine` 或 file-based `KvStore`，可用不同线程池实现）。
// 2. 并发模型详解：
//    - 调用 `run` 时，服务器会在给定地址上 `bind` 并监听连接。
//    - 对于每个进入的连接（`stream`），我们克隆 `engine`（要求 `engine` 实现 `Clone`），并将处理逻辑提交给线程池：
//        `self.pool.spawn(move || { ... })`。线程池决定如何调度这个任务（直接新建线程、从队列取任务、或者 rayon 的调度）。
//    - 在任务中，会调用 `serve(engine, stream)` 执行具体的请求读取与响应逻辑。
// 3. 为什么要 `clone` 引擎：
//    - 每个连接都在独立的任务（线程）中处理，若 `engine` 包含内部共享的状态（比如 `Arc<Mutex<...>>`），克隆通常只是复制 `Arc`，不会复制实际数据，
//      因此多个任务可以并发访问同一个底层资源（需要内部同步）。因此，实现 `KvsEngine` 时通常会用 `Arc` 等类型来保证安全共享。
// 4. serve 函数如何工作：
//    - 使用 `BufReader` 从 TCP 流中通过 `serde_json::Deserializer` 解析一系列 `Request`（基于 stream 的 JSON 解析）。
//    - 对于每个 `Request`，根据类型调用 `engine.get/set/remove`，并通过 `serde_json::to_writer` 将响应写回客户端。
//    - 使用宏 `send_resp!` 统一序列化与 flush，保证在发送每个响应后数据被推进到网络。
// 5. 错误与健壮性考虑：
//    - 连接级别出错或请求反序列化出错会导致该连接的任务返回错误，但不会影响其他连接（错误被记录）。
//    - 如果 `engine` 在并发访问时使用 `Mutex`，要注意不要在持锁状态下进行阻塞或长时间 IO，以免影响其他请求。
// 6. 新手建议：
//    - 先在单线程下跑通整个请求-响应逻辑（例如把 `pool.spawn` 改为直接调用 `serve`），理解 IO 与 serde 的配合；
//    - 再切换到线程池并观察并发与竞态问题，使用 `Arc<Mutex<...>>` 或无锁结构（如 `SkipMap`）来优化并发访问；
//    - 使用日志（`log` crate）记录关键操作与错误，方便调试并发场景下的问题。
