use crate::common::{GetResponse, RemoveResponse, Request, SetResponse};
use crate::{KvsError, Result};
use serde::Deserialize;
use serde_json::de::{Deserializer, IoRead};
use std::io::{BufReader, BufWriter, Write};
use std::net::{TcpStream, ToSocketAddrs};

/// Key value store client
pub struct KvsClient {
    reader: Deserializer<IoRead<BufReader<TcpStream>>>,
    writer: BufWriter<TcpStream>,
}

// 详细中文注释（补充）：
// 1. `KvsClient` 的职责：作为同步（阻塞）客户端连接到 `KvsServer`，发送 `Request` 并读取 `Response`。
// 2. 读写分工：
//    - `writer`：负责将 `Request` 通过 `serde_json::to_writer` 序列化并写入 TCP 流，然后 `flush()` 将数据发送到服务器。
//    - `reader`：使用 `serde_json::de::Deserializer` 的流式解析器从 TCP 流中反序列化响应，这样可以从同一连接连续读取多个响应。
// 3. 同步/阻塞语义：
//    - 该客户端是同步设计，所有方法（`get/set/remove`）都会阻塞直到完成网络往返（写入请求并读取响应）。
//    - 对于需要高并发的场景，应考虑使用异步客户端或在外部使用线程池进行并发调用。
// 4. 错误处理与语义：
//    - 服务端通过 `GetResponse::Err(String)` 等将业务错误（例如 key not found）传回，客户端将其转换为 `KvsError::StringError`。
//    - 网络错误或反序列化错误会被转换为 `KvsError` 并上抛给调用者。
// 5. 对 Rust 新手的建议：
//    - 注意 `TcpStream::try_clone()`：它并不复制底层连接，而是创建一个共享句柄，读写可以分开处理（本例将读、写句柄分别包装）。
//    - `Deserializer::from_reader` 是流式的 JSON 解析器，适合在二进制流中连续读取多个 JSON 值而不必把整个响应读到内存。

impl KvsClient {
    /// Connect to `addr` to access `KvsServer`.
    pub fn connect<A: ToSocketAddrs>(addr: A) -> Result<Self> {
        let tcp_reader = TcpStream::connect(addr)?;
        let tcp_writer = tcp_reader.try_clone()?;
        Ok(KvsClient {
            reader: Deserializer::from_reader(BufReader::new(tcp_reader)),
            writer: BufWriter::new(tcp_writer),
        })
    }

    /// Get the value of a given key from the server.
    pub fn get(&mut self, key: String) -> Result<Option<String>> {
        serde_json::to_writer(&mut self.writer, &Request::Get { key })?;
        self.writer.flush()?;
        let resp = GetResponse::deserialize(&mut self.reader)?;
        match resp {
            GetResponse::Ok(value) => Ok(value),
            GetResponse::Err(msg) => Err(KvsError::StringError(msg)),
        }
    }

    /// Set the value of a string key in the server.
    pub fn set(&mut self, key: String, value: String) -> Result<()> {
        serde_json::to_writer(&mut self.writer, &Request::Set { key, value })?;
        self.writer.flush()?;
        let resp = SetResponse::deserialize(&mut self.reader)?;
        match resp {
            SetResponse::Ok(_) => Ok(()),
            SetResponse::Err(msg) => Err(KvsError::StringError(msg)),
        }
    }

    /// Remove a string key in the server.
    pub fn remove(&mut self, key: String) -> Result<()> {
        serde_json::to_writer(&mut self.writer, &Request::Remove { key })?;
        self.writer.flush()?;
        let resp = RemoveResponse::deserialize(&mut self.reader)?;
        match resp {
            RemoveResponse::Ok(_) => Ok(()),
            RemoveResponse::Err(msg) => Err(KvsError::StringError(msg)),
        }
    }
}
