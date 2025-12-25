use crate::common::{Request, Response};
use crate::KvsError;
use std::net::SocketAddr;
use tokio::codec::{FramedRead, FramedWrite, LengthDelimitedCodec};
use tokio::io::{ReadHalf, WriteHalf};
use tokio::net::TcpStream;
use tokio::prelude::*;
use tokio_serde_json::{ReadJson, WriteJson};

/// 键值存储客户端，使用异步 I/O 与服务器交互
pub struct KvsClient {
    // 用于读取并解析 JSON 响应的流
    read_json: ReadJson<FramedRead<ReadHalf<TcpStream>, LengthDelimitedCodec>, Response>,
    // 用于序列化并发送 JSON 请求的流
    write_json: WriteJson<FramedWrite<WriteHalf<TcpStream>, LengthDelimitedCodec>, Request>,
}

impl KvsClient {
    /// 连接到指定的地址以访问 `KvsServer`。
    /// 返回一个 Future，完成后提供 KvsClient 实例。
    pub fn connect(addr: SocketAddr) -> impl Future<Item = Self, Error = KvsError> {
        TcpStream::connect(&addr)
            .map(|tcp| {
                // 将 TCP 流拆分为读写两部分，以便并行或交替处理
                let (read_half, write_half) = tcp.split();
                // 使用 LengthDelimitedCodec 处理长度前缀，ReadJson 处理 JSON 解码
                let read_json =
                    ReadJson::new(FramedRead::new(read_half, LengthDelimitedCodec::new()));
                // 使用 LengthDelimitedCodec 处理长度前缀，WriteJson 处理 JSON 编码
                let write_json =
                    WriteJson::new(FramedWrite::new(write_half, LengthDelimitedCodec::new()));
                KvsClient {
                    read_json,
                    write_json,
                }
            })
            .map_err(|e| e.into())
    }

    /// 从服务器获取给定键的值。
    /// 这里的 API 设计采用了消耗 self 并返回 (Value, Self) 的模式，以符合异步所有权模型。
    pub fn get(self, key: String) -> impl Future<Item = (Option<String>, Self), Error = KvsError> {
        self.send_request(Request::Get { key })
            .and_then(move |(resp, client)| match resp {
                Some(Response::Get(value)) => Ok((value, client)),
                Some(Response::Err(msg)) => Err(KvsError::StringError(msg)),
                Some(_) => Err(KvsError::StringError("Invalid response".to_owned())),
                None => Err(KvsError::StringError("No response received".to_owned())),
            })
    }

    /// 在服务器中设置字符串键的值。
    pub fn set(self, key: String, value: String) -> impl Future<Item = Self, Error = KvsError> {
        self.send_request(Request::Set { key, value })
            .and_then(move |(resp, client)| match resp {
                Some(Response::Set) => Ok(client),
                Some(Response::Err(msg)) => Err(KvsError::StringError(msg)),
                Some(_) => Err(KvsError::StringError("Invalid response".to_owned())),
                None => Err(KvsError::StringError("No response received".to_owned())),
            })
    }

    /// 移除服务器中的字符串键。
    pub fn remove(self, key: String) -> impl Future<Item = Self, Error = KvsError> {
        self.send_request(Request::Remove { key })
            .and_then(move |(resp, client)| match resp {
                Some(Response::Remove) => Ok(client),
                Some(Response::Err(msg)) => Err(KvsError::StringError(msg)),
                Some(_) => Err(KvsError::StringError("Invalid response".to_owned())),
                None => Err(KvsError::StringError("No response received".to_owned())),
            })
    }

    /// 内部方法：发送请求并异步等待响应。
    fn send_request(
        self,
        req: Request,
    ) -> impl Future<Item = (Option<Response>, Self), Error = KvsError> {
        let read_json = self.read_json;
        self.write_json
            .send(req) // 发送请求
            .and_then(move |write_json| {
                read_json
                    .into_future() // 获取响应流中的下一个值
                    .map(move |(resp, read_json)| {
                        let client = KvsClient {
                            read_json,
                            write_json,
                        };
                        (resp, client)
                    })
                    .map_err(|(err, _)| err)
            })
            .map_err(|e| e.into())
    }
}
