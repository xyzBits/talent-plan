use clap::{App, AppSettings, Arg, SubCommand};
use kvs::{KvStore, KvsError, Result};
use std::env::current_dir;
use std::process::exit;

fn main() -> Result<()> {
    // 配置命令行参数
    let matches = App::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .setting(AppSettings::DisableHelpSubcommand)
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .setting(AppSettings::VersionlessSubcommands)
        .subcommand(
            SubCommand::with_name("set")
                .about("设置一个字符串键的值")
                .arg(Arg::with_name("KEY").help("字符串键").required(true))
                .arg(
                    Arg::with_name("VALUE")
                        .help("该键对应的字符串值")
                        .required(true),
                ),
        )
        .subcommand(
            SubCommand::with_name("get")
                .about("获取给定字符串键的值")
                .arg(Arg::with_name("KEY").help("字符串键").required(true)),
        )
        .subcommand(
            SubCommand::with_name("rm")
                .about("移除给定的键")
                .arg(Arg::with_name("KEY").help("字符串键").required(true)),
        )
        .get_matches();

    // 根据子命令执行相应操作
    match matches.subcommand() {
        ("set", Some(matches)) => {
            let key = matches.value_of("KEY").unwrap();
            let value = matches.value_of("VALUE").unwrap();

            let mut store = KvStore::open(current_dir()?)?;
            store.set(key.to_string(), value.to_string())?;
        }
        ("get", Some(matches)) => {
            let key = matches.value_of("KEY").unwrap();

            let mut store = KvStore::open(current_dir()?)?;
            if let Some(value) = store.get(key.to_string())? {
                println!("{}", value);
            } else {
                println!("Key not found");
            }
        }
        ("rm", Some(matches)) => {
            let key = matches.value_of("KEY").unwrap();

            let mut store = KvStore::open(current_dir()?)?;
            match store.remove(key.to_string()) {
                Ok(()) => {}
                Err(KvsError::KeyNotFound) => {
                    println!("Key not found");
                    exit(1);
                }
                Err(e) => return Err(e),
            }
        }
        _ => unreachable!(),
    }
    Ok(())
}
