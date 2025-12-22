# PNA Rust 项目 1: Rust 工具箱

**任务**：创建一个内存键值存储，能够通过命令行参数进行简单测试。

**目标**：
- 安装 Rust 编译器和工具
- 学习本课程中使用的项目结构
- 使用 `cargo init` / `run` / `test` / `clippy` / `fmt`
- 学会从 crates.io 查找并导入 crates
- 为键值存储定义合适的数据类型

**主题**：测试、`clap` crate、`CARGO_VERSION` 等，`clippy` 与 `rustfmt` 工具。

**扩展**：`structopt` crate。

- [简介](#user-content-introduction)
- [项目规范](#user-content-project-spec)
- [安装](#user-content-installation)
- [项目设置](#user-content-project-setup)
- [第 1 部分：让测试编译通过](#user-content-part-1-make-the-tests-compile)
  - [附注：测试技巧](#user-content-aside-testing-tips)
- [第 2 部分：接受命令行参数](#user-content-part-2-accept-command-line-arguments)
- [第 3 部分：Cargo 环境变量](#user-content-part-3-cargo-environment-variables)
- [第 4 部分：在内存中存储值](#user-content-part-4-store-values-in-memory)
- [第 5 部分：文档](#user-content-part-5-documentation)
- [第 6 部分：使用 `clippy` 与 `rustfmt` 保持良好代码风格](#user-content-part-6-ensure-good-style-with-clippy-and-rustfmt)
- [扩展 1：`structopt`](#user-content-extension-1-structopt)

## 简介

在本项目中，你将创建一个简单的内存键值存储，将 **字符串映射到字符串**，并通过一些测试。重点在于工具链和项目设置，而不是业务逻辑本身。

如果你觉得这太基础，请仍然完成本项目，因为它会涉及本课程后续会使用的通用模式。

## 项目规范

Cargo 项目 `kvs` 会构建一个名为 `kvs` 的命令行客户端，调用库 `kvs`。

`kvs` 可执行文件支持以下命令行参数：

- `kvs set <KEY> <VALUE>`   设置键值对
- `kvs get <KEY>`            获取键对应的值
- `kvs rm <KEY>`             删除键
- `kvs -V`                   打印版本号

库 `kvs` 包含一个 `KvStore` 类型，提供以下方法：

- `KvStore::set(&mut self, key: String, value: String)`
- `KvStore::get(&self, key: String) -> Option<String>`
- `KvStore::remove(&mut self, key: String)`

`KvStore` 只在内存中保存数据，因此命令行客户端的功能非常有限，`get`/`set`/`rm` 在运行时会返回 `unimplemented` 错误。

## 安装

在你的 Rust 编程经验中，你应该已经了解如何通过 [rustup] 安装 Rust。

如果还没有，请执行以下命令（在 Windows 上请参考 rustup.rs 上的说明）：

```bash
curl https://sh.rustup.rs -sSf | sh
```

验证工具链是否可用：`rustc -V`。

## 项目设置

你将在自己的 Git 仓库中完成本项目。可以使用 `cargo new --lib`、`cargo init --lib`，或手动创建目录结构。

项目目录结构示例：

```
├── Cargo.toml
├── src
│   ├── bin
│   │   └── kvs.rs
│   └── lib.rs
└── tests
    └── tests.rs
```

`Cargo.toml`、`lib.rs` 与 `kvs.rs` 内容如下（仅示例）：

`Cargo.toml`：

```toml
[package]
name = "kvs"
version = "0.1.0"
authors = ["Your Name <you@example.com>"]
description = "A key-value store"
edition = "2018"
```

`lib.rs`：

```rust
// 暂时留空
```

`kvs.rs`：

```rust
fn main() {
    println!("Hello, world!");
}
```

确保项目名称为 `kvs`，因为测试用例会依据此名称进行链接。

## 第 1 部分：让测试编译通过

打开 `tests/tests.rs`，阅读测试用例。先在 `src/lib.rs` 中写出所有需要的类型和方法的 **声明**（实现体可以先写 `panic!()`），使得 `cargo test --no-run` 能成功编译。

随后运行 `cargo test`，会看到大量测试失败，这正是我们接下来要实现的目标。

### 附注：测试技巧

- `cargo test --lib` 只运行库内部的测试。
- `cargo test --doc` 运行文档中的示例测试。
- `cargo test --bins` 运行所有二进制的测试。
- `cargo test --bin foo` 只运行名为 `foo` 的二进制。
- `cargo test --test foo` 只运行 `tests/foo.rs` 中的测试。

了解这些选项可以帮助你在调试时只运行感兴趣的测试。

## 第 2 部分：接受命令行参数

使用 `clap` crate 解析命令行参数。请在 `Cargo.toml` 中加入最新版本的 `clap`（可通过 `cargo search clap` 或 `cargo add clap`）。实现 CLI，使得 `cli_*` 系列测试通过。提示：在未实现功能时，`get`、`set`、`rm` 应向 `stderr` 打印 `unimplemented` 并返回非零退出码。

运行方式示例：

```bash
cargo run -- get key1
```

## 第 3 部分：Cargo 环境变量

在 `clap` 配置中，使用 Cargo 自动提供的环境变量（如 `CARGO_PKG_NAME`、`CARGO_PKG_VERSION`）来填充程序的名称、版本、作者等信息，避免在代码中硬编码这些值。

## 第 4 部分：在内存中存储值

实现 `KvStore` 的内部数据结构（如 `HashMap<String, String>`），并完成 `set`、`get`、`remove` 的具体逻辑，使得所有剩余的测试全部通过。

## 第 5 部分：文档

为公开的 API 添加文档注释（`///`），并在 `src/lib.rs` 顶部加入 `#![deny(missing_docs)]`，强制所有公共项都有文档。文档中应包含使用示例，以便 `cargo test --doc` 能验证示例代码。

## 第 6 部分：使用 `clippy` 与 `rustfmt` 保持良好代码风格

- 安装组件：`rustup component add clippy`、`rustup component add rustfmt`。
- 运行 `cargo clippy` 并根据提示修复警告。
- 运行 `cargo fmt` 自动格式化代码。

完成以上步骤后，你的项目已经具备了基本的可用性、良好的文档以及一致的代码风格。

---

**祝贺**，你已经完成了本项目的第一轮开发！如果有兴趣，可以继续完成 **扩展 1：`structopt`**，使用更简洁的方式定义命令行参数。

<!-- 这里可以加入更多关于 Rust 工具链的探索内容 -->
