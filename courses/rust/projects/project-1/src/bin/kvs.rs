#!/usr/bin/env rust
/// 命令行入口，使用 `clap` 解析子命令 `set`、`get`、`rm`。
/// 当前实现仅在未实现功能时打印 `unimplemented` 并退出，后续可自行实现业务逻辑。
use clap::{App, AppSettings, Arg, SubCommand};
use std::process::exit;
use kvs::KvStore;

fn main() {
    // 使用 Cargo 提供的环境变量自动填充程序信息
    let matches = App::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .setting(AppSettings::DisableHelpSubcommand)
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .setting(AppSettings::VersionlessSubcommands)
        // 子命令: set <KEY> <VALUE>
        .subcommand(
            SubCommand::with_name("set")
                .about("设置键值对：set <KEY> <VALUE>")
                .arg(Arg::with_name("KEY").help("键名").required(true))
                .arg(Arg::with_name("VALUE").help("键值").required(true)),
        )
        // 子命令: get <KEY>
        .subcommand(
            SubCommand::with_name("get")
                .about("获取键对应的值：get <KEY>")
                .arg(Arg::with_name("KEY").help("键名").required(true)),
        )
        // 子命令: rm <KEY>
        .subcommand(
            SubCommand::with_name("rm")
                .about("删除键：rm <KEY>")
                .arg(Arg::with_name("KEY").help("键名").required(true)),
        )
        .get_matches();

    let store = KvStore::new();

    // 根据匹配的子命令执行对应逻辑（目前占位）
    match matches.subcommand() {
        ("set", Some(_)) => {
            eprintln!("unimplemented");
            exit(1);
        }
        ("get", Some(_)) => {
            eprintln!("unimplemented");
            exit(1);
        }
        ("rm", Some(_)) => {
            eprintln!("unimplemented");
            exit(1);
        }
        _ => unreachable!(),
    }
}
