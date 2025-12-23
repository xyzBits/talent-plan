use clap::AppSettings;
use kvs::{KvsClient, Result};
use std::net::SocketAddr;
use std::process::exit;
use structopt::StructOpt;

const DEFAULT_LISTENING_ADDRESS: &str = "127.0.0.1:4000";
const ADDRESS_FORMAT: &str = "IP:PORT";

/// kvs-client 的命令行参数结构
#[derive(StructOpt, Debug)]
#[structopt(
    name = "kvs-client",
    raw(global_settings = "&[\
                           AppSettings::DisableHelpSubcommand,\
                           AppSettings::VersionlessSubcommands]")
)]
struct Opt {
    /// 具体的子命令
    #[structopt(subcommand)]
    command: Command,
}

/// kvs-client 支持的子命令
#[derive(StructOpt, Debug)]
enum Command {
    /// 获取给定键的值
    #[structopt(name = "get", about = "Get the string value of a given string key")]
    Get {
        /// 键名称
        #[structopt(name = "KEY", help = "A string key")]
        key: String,
        /// 服务器地址
        #[structopt(
            long,
            help = "Sets the server address",
            raw(value_name = "ADDRESS_FORMAT"),
            raw(default_value = "DEFAULT_LISTENING_ADDRESS"),
            parse(try_from_str)
        )]
        addr: SocketAddr,
    },
    /// 设置给定键的值
    #[structopt(name = "set", about = "Set the value of a string key to a string")]
    Set {
        /// 键名称
        #[structopt(name = "KEY", help = "A string key")]
        key: String,
        /// 值名称
        #[structopt(name = "VALUE", help = "The string value of the key")]
        value: String,
        /// 服务器地址
        #[structopt(
            long,
            help = "Sets the server address",
            raw(value_name = "ADDRESS_FORMAT"),
            raw(default_value = "DEFAULT_LISTENING_ADDRESS"),
            parse(try_from_str)
        )]
        addr: SocketAddr,
    },
    /// 移除给定的键
    #[structopt(name = "rm", about = "Remove a given string key")]
    Remove {
        /// 键名称
        #[structopt(name = "KEY", help = "A string key")]
        key: String,
        /// 服务器地址
        #[structopt(
            long,
            help = "Sets the server address",
            raw(value_name = "ADDRESS_FORMAT"),
            raw(default_value = "DEFAULT_LISTENING_ADDRESS"),
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
