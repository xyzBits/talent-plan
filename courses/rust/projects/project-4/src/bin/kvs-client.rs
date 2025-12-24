use clap::AppSettings;
use kvs::{KvsClient, Result};
use std::net::SocketAddr;
use std::process::exit;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(
    name = "kvs-client",
    raw(global_settings = "&[\
                           AppSettings::DisableHelpSubcommand,\
                           AppSettings::VersionlessSubcommands]")
)]
struct Opt {
    #[structopt(subcommand)]
    command: Command,
}

#[derive(StructOpt, Debug)]
enum Command {
    #[structopt(name = "get", about = "Get the string value of a given string key")]
    Get {
        #[structopt(name = "KEY", help = "A string key")]
        key: String,
        #[structopt(
            long,
            help = "Sets the server address",
            value_name = "IP:PORT",
            default_value = "127.0.0.1:4000",
            parse(try_from_str)
        )]
        addr: SocketAddr,
    },
    #[structopt(name = "set", about = "Set the value of a string key to a string")]
    Set {
        #[structopt(name = "KEY", help = "A string key")]
        key: String,
        #[structopt(name = "VALUE", help = "The string value of the key")]
        value: String,
        #[structopt(
            long,
            help = "Sets the server address",
            value_name = "IP:PORT",
            default_value = "127.0.0.1:4000",
            parse(try_from_str)
        )]
        addr: SocketAddr,
    },
    #[structopt(name = "rm", about = "Remove a given string key")]
    Remove {
        #[structopt(name = "KEY", help = "A string key")]
        key: String,
        #[structopt(
            long,
            help = "Sets the server address",
            value_name = "IP:PORT",
            default_value = "127.0.0.1:4000",
            parse(try_from_str)
        )]
        addr: SocketAddr,
    },
}

fn main() {
    let opt = Opt::from_args();
    if let Err(e) = run(opt) {
        eprintln!("{}", e);
        exit(1);
    }
}

// 详细中文注释（补充）：
// 1. CLI 行为概述：
//    - `kvs-client` 提供三个子命令：`get`、`set`、`rm`，分别对应对远端 `KvsServer` 的三种操作。
//    - 每个子命令都接受一个可选的 `--addr` 参数，用来指定服务器地址；默认地址为 `127.0.0.1:4000`，便于本地调试。
// 2. 错误处理语义：
//    - 主函数捕获 `run` 返回的 `Result`，如果有错误则打印到标准错误并以非零状态退出；这在脚本或 CI 中很方便。
// 3. 对新手的建议：
//    - 如果想对请求进行并发化或非阻塞处理，可以在外层使用线程池来并行执行 `KvsClient::connect` + 请求序列，
//      或者使用异步版本的客户端（本项目中较新的项目采用 Tokio/async 模型）。

fn run(opt: Opt) -> Result<()> {
    match opt.command {
        Command::Get { key, addr } => {
            let mut client = KvsClient::connect(addr)?;
            if let Some(value) = client.get(key)? {
                println!("{}", value);
            } else {
                println!("Key not found");
            }
        }
        Command::Set { key, value, addr } => {
            let mut client = KvsClient::connect(addr)?;
            client.set(key, value)?;
        }
        Command::Remove { key, addr } => {
            let mut client = KvsClient::connect(addr)?;
            client.remove(key)?;
        }
    }
    Ok(())
}
