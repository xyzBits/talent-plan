use clap::arg_enum;
use kvs::thread_pool::*;
use kvs::*;
use log::LevelFilter;
use log::{error, info, warn};
use std::env;
use std::env::current_dir;
use std::fs;
use std::net::SocketAddr;
use std::process::exit;
use structopt::StructOpt;

const DEFAULT_LISTENING_ADDRESS: &str = "127.0.0.1:4000";
const DEFAULT_ENGINE: Engine = Engine::kvs;

#[derive(StructOpt, Debug)]
#[structopt(name = "kvs-server")]
struct Opt {
    #[structopt(
        long,
        help = "Sets the listening address",
        value_name = "IP:PORT",
        raw(default_value = "DEFAULT_LISTENING_ADDRESS"),
        parse(try_from_str)
    )]
    addr: SocketAddr,
    #[structopt(
        long,
        help = "Sets the storage engine",
        value_name = "ENGINE-NAME",
        raw(possible_values = "&Engine::variants()")
    )]
    engine: Option<Engine>,
}

arg_enum! {
    #[allow(non_camel_case_types)]
    #[derive(Debug, Copy, Clone, PartialEq, Eq)]
    enum Engine {
        kvs,
        sled
    }
}

fn main() {
    env_logger::builder()
        .filter_level(LevelFilter::Info)
        .init();
    let mut opt = Opt::from_args();
    // 详细中文注释（补充）：
    // 1. 入口流程概览：
    //    - 使用 `StructOpt`/`clap` 解析命令行参数（监听地址和存储引擎），然后根据配置选择存储引擎并运行服务器。
    //    - 将选定的 engine 写入当前目录下的 `engine` 文件，供后续运行或调试参考。
    // 2. 关于 `Engine` 选择与兼容性：
    //    - 程序在不同次启动之间不允许随意切换 engine（如果 `engine` 文件存在且与当前运行时选项不一致，则退出）。
    //    - 这是为了避免不同引擎在同一目录下写入互不兼容的数据文件从而导致数据损坏。
    // 3. 日志与调试：
    //    - 使用 `env_logger` 并设置 `Info` 级别，允许通过环境变量调整日志级别以便调试。
    let res = current_engine().and_then(move |curr_engine| {
        if opt.engine.is_none() {
            opt.engine = curr_engine;
        }
        if curr_engine.is_some() && opt.engine != curr_engine {
            error!("Wrong engine!");
            exit(1);
        }
        run(opt)
    });
    if let Err(e) = res {
        error!("{}", e);
        exit(1);
    }
}

fn run(opt: Opt) -> Result<()> {
    let engine = opt.engine.unwrap_or(DEFAULT_ENGINE);
    info!("kvs-server {}", env!("CARGO_PKG_VERSION"));
    info!("Storage engine: {}", engine);
    info!("Listening on {}", opt.addr);

    // write engine to engine file
    fs::write(current_dir()?.join("engine"), format!("{}", engine))?;

    let pool = RayonThreadPool::new(num_cpus::get() as u32)?;

    match engine {
        Engine::kvs => run_with(KvStore::open(env::current_dir()?)?, pool, opt.addr),
        Engine::sled => run_with(
            SledKvsEngine::new(sled::open(env::current_dir()?)?),
            pool,
            opt.addr,
        ),
    }
}

pub fn run_with<E: KvsEngine, P: ThreadPool>(engine: E, pool: P, addr: SocketAddr) -> Result<()> {
    let server = KvsServer::new(engine, pool);
    server.run(addr)
}

fn current_engine() -> Result<Option<Engine>> {
    let engine = current_dir()?.join("engine");
    if !engine.exists() {
        return Ok(None);
    }

    match fs::read_to_string(engine)?.parse() {
        Ok(engine) => Ok(Some(engine)),
        Err(e) => {
            warn!("The content of engine file is invalid: {}", e);
            Ok(None)
        }
    }
}
