use crate::common::{Request, Response};
use crate::{KvsEngine, KvsError, Result};
use std::net::SocketAddr;
use tokio::codec::{FramedRead, FramedWrite, LengthDelimitedCodec};
use tokio::net::{TcpListener, TcpStream};
use tokio::prelude::*;
use tokio_serde_json::{ReadJson, WriteJson};

/// 键值存储服务器，使用指定的存储引擎处理请求
pub struct KvsServer<E: KvsEngine> {
    engine: E,
}

impl<E: KvsEngine> KvsServer<E> {
    /// 使用给定的存储引擎创建一个 `KvsServer` 实例。
    pub fn new(engine: E) -> Self {
        KvsServer { engine }
    }

    /// 在给定地址上运行服务器并进行监听。
    pub fn run(self, addr: SocketAddr) -> Result<()> {
        // 绑定监听地址
        let listener = TcpListener::bind(&addr)?;
        // 创建服务器 Future，处理传入的 TCP 连接
        let server = listener
            .incoming() // 获取 TCP 连接流
            .map_err(|e| error!("IO error: {}", e))
            .for_each(move |tcp| {
                // 为每个连接克隆一份引擎引用，并在异步任务中处理
                let engine = self.engine.clone();
                serve(engine, tcp).map_err(|e| error!("Error on serving client: {}", e))
            });
        // 启动 tokio 运行时驱动服务器运行
        tokio::run(server);
        Ok(())
    }
}

/// 内部函数：处理单个客户端连接。
fn serve<E: KvsEngine>(engine: E, tcp: TcpStream) -> impl Future<Item = (), Error = KvsError> {
    // 拆分 TCP 流以便独立读写
    let (read_half, write_half) = tcp.split();
    // 设置读 JSON 的适配层
    let read_json = ReadJson::new(FramedRead::new(read_half, LengthDelimitedCodec::new()));
    
    // 创建响应流：读取请求 -> 使用引擎处理 -> 映射为响应
    let resp_stream = read_json
        .map_err(KvsError::from)
        .and_then(
            move |req| -> Box<dyn Future<Item = Response, Error = KvsError> + Send> {
                match req {
                    Request::Get { key } => Box::new(engine.get(key).map(Response::Get)),
                    Request::Set { key, value } => {
                        Box::new(engine.set(key, value).map(|_| Response::Set))
                    }
                    Request::Remove { key } => {
                        Box::new(engine.remove(key).map(|_| Response::Remove))
                    }
                }
            },
        )
        // 处理可能发生的错误，并将其包装在 Response::Err 中返回给客户端，而不是直接终止连接
        .then(|resp| -> Result<Response> {
            match resp {
                Ok(resp) => Ok(resp),
                Err(e) => Ok(Response::Err(format!("{}", e))),
            }
        });

    // 设置写 JSON 的适配层
    let write_json = WriteJson::new(FramedWrite::new(write_half, LengthDelimitedCodec::new()));
    // 将整个响应流发送回客户端
    write_json
        .sink_map_err(KvsError::from)
        .send_all(resp_stream)
        .map(|_| ())
}
