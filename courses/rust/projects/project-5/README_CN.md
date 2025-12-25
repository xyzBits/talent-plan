# 项目：异步 (Asynchrony)

**任务**：创建一个多线程、持久化的键值存储服务器和客户端，使用自定义协议进行“异步”网络通信。

**目标**：

- 理解编写 Rust futures 时使用的模式
- 理解 futures 的错误处理
- 学习调试类型系统
- 使用 tokio 运行时进行异步网络通信
- 使用 boxed futures 处理复杂的类型系统问题
- 使用 `impl Trait` 创建匿名 `Future` 类型

**主题**：异步 (asynchrony), futures, tokio, `impl Trait`。

**扩展**：tokio-fs。

- [简介](#简介)
- [项目规范](#项目规范)
- [项目设置](#项目设置)
- [背景：在 Rust 中使用 futures 思考](#背景在-rust-中使用-futures-思考)
- [第 1 部分：将 tokio 引入客户端](#第-1-部分将-tokio-引入客户端)
- [第 2 部分：将 `KvsClient` 转换为 boxed futures](#第-2-部分将-kvsclient-转换为-boxed-futures)
- [第 3 部分：使用显式 future 类型的 `KvsClient`](#第-3-部分使用显式-future-类型的-kvsclient)
- [第 4 部分：使用匿名 future 类型的 `KvsClient`](#第-4-部分使用匿名-future-类型的-kvsclient)
- [第 5 部分：使 `ThreadPool` 可共享](#第-5-部分使-threadpool-可共享)
- [第 6 部分：将 `KvsEngine` 转换为 futures](#第-6-部分将-kvsengine-转换为-futures)
- [第 7 部分：使用 tokio 驱动 `KvsEngine`](#第-7-部分使用-tokio-驱动-kvsengine)
- [扩展 1：转换为 tokio-fs](#扩展-1转换为-tokio-fs)


## 简介

_注意：本项目目前仅有大纲，尚未完成编写。如果您在课程中进行到此，请发送邮件至 brian@pingcap.com 告知我，我会尽快完成编写。_

在这个项目中，您将创建一个简单的键值服务器和客户端，它们通过自定义协议进行通信。服务器将使用基于 tokio 运行时的异步网络。负责读写文件的键值引擎将保持同步，在底层的线程池中调度工作，同时对外呈现异步接口。在此过程中，您将尝试多种定义和使用 future 类型的方法。

因为学习使用 Rust futures 编程特别具有挑战性，且现有的相关文档有限，所以本项目的范围相对较小，并且包含了比以往项目更多的直接解释。

请务必阅读本项目的背景资料。如果您感到挫败，请原谅自己，休息一下，换个心情再试一次。编写异步 Rust 对每个人来说都很困难。


## 项目规范

cargo 项目 `kvs` 构建了一个名为 `kvs-client` 的命令行键值存储客户端和一个名为 `kvs-server` 的键值存储服务器，两者都会调用名为 `kvs` 的库。客户端通过自定义协议与服务器通信。

CLI 的接口与[上一个项目]相同。引擎的实现也基本相同，通过线程池分发同步文件 I/O。

这次的不同之处在于，所有的网络操作都是异步执行的。

作为异步转换的一部分，`KvsClient` 将提供基于 futures 的 API，`KvsEngine` trait 也将提供基于 futures 的 API，即使它是通过线程池使用阻塞（同步）I/O 实现的。

您的 `KvsServer` 将基于 tokio 运行时，它会自动将异步工作分发到多个线程（tokio 本身包含一个线程池）。这意味着您的架构实际上会有两层线程池：第一层用于异步处理网络，每个核心一个线程；第二层用于同步处理文件 I/O，拥有足够的线程以使网络线程尽可能保持忙碌。

由于这种架构变化，您的任务将从多个线程被派发到您的线程池中，因此您的 `ThreadPool` trait 及其实现将变成实现了 `Clone + Send + 'sync` 的共享类型，就像您的 `KvsEngine` 一样。

因为您将尝试这些类型返回的 futures 的多种定义，所以这里没有完全具体说明，而是在需要时再进行规定。

更具体地说，您将处理如下所示的函数签名：

- `Client::get(&mut self, key: String) -> Box<Future<Item = Option<String>, Error = Error>`

- `Client::get(&mut self, key: String) -> future::SomeExplicitCombinator<...>`

- `Client::get(&mut self, key: String) -> impl Future<Item = Option<String>, Error = Error>`

- `Client::get(&mut self, key: String) -> ClientGetFuture`



## 项目设置

接续上一个项目，删除之前的 `tests` 目录，并将本项目的 `tests` 目录复制到该位置。本项目应包含一个名为 `kvs` 的库，以及两个可执行文件 `kvs-server` 和 `kvs-client`。

您需要在 `Cargo.toml` 中添加以下开发依赖 (dev-dependencies)：

```toml
[dev-dependencies]
assert_cmd = "0.11"
criterion = "0.2.11"
crossbeam-utils = "0.6.5"
predicates = "1.0.0"
rand = "0.6.5"
tempfile = "3.0.7"
walkdir = "2.2.7"
panic-control = "0.1.4"
```

与之前的项目不同，不必急于填补足够的类型定义来使测试套件通过编译。这样做需要一次性跨越多个步骤。正文会指明何时开始处理测试套件。


## 背景：在 Rust 中使用 futures 思考

- 为什么使用 futures？网络 vs 文件/IO，阻塞 vs 非阻塞，同步 vs 异步
- 从用户角度看 futures（而不是以 poll 为中心的实现角度）
- 不要对执行器 (executors) 和运行时 (runtimes) 考虑过多
- 方法链以及它如何转换 future 类型
- 调试 Rust 类型
- Result vs Future vs FutureResult
- 使用 futures 进行错误处理
- 具体 futures vs Boxed futures vs 匿名 futures
- 关于 futures 0.1 和 futures 0.3 的说明（我们将使用 futures 0.1）
- 关于 async / await 的说明


## 第 1 部分：将 tokio 引入客户端

最终我们将把客户端和服务器都转换为异步，但由于客户端非常简单，我们将从那里开始。我们将首先引入 tokio 运行时，同时继续使用现有的同步 `KvsClient`。

对于客户端，我们将引入异步运行时，同时保留同步的 `KvsClient`，然后转换 `KvsClient`。`KvsClient` 的 `connect` 方法。请注意，作为一个库，`KvsClient` 基于 futures 可以提供最高的效率，但我们的 `kvs-client` 二进制文件并未利用这一点，因此该二进制文件运行单个 future 然后退出的样子可能看起来有点滑稽。

TODO @sticnarf - 看看是否可以编写与具体 future 类型无关的测试用例，以便它们适用于以下所有策略。


## 第 2 部分：将 `KvsClient` 转换为 boxed futures

未来类型阻力最小的路径。


## 第 3 部分：使用显式 future 类型的 `KvsClient`

仅仅是为了体验一下这种方式是多么的难以维持。


## 第 4 部分：使用匿名 future 类型的 `KvsClient`

最终解决方案。


## 第 5 部分：使 `ThreadPool` 可共享


## 第 6 部分：将 `KvsEngine` 转换为 futures

对于服务器，我们要做的与客户端相反，即为 `KvsEngine` 提供异步接口。这将表明 futures 和底层运行时是独立的，并提供更广泛的经验。


## 第 7 部分：使用 tokio 驱动 `KvsEngine`

请注意，尽管我们自己编写的异步代码很少，但 tokio 本身正在将异步工作分发到多个线程。思考一下将 CPU 密集型工作直接放在网络线程或文件线程上的权衡，例如，序列化操作应该放在哪里？

TODO

编写得不错，朋友。好好休息一下。

---

## 扩展 1：转换为 tokio-fs

不确定这应该是必选要求还是扩展。

<!--
TODO:
- 我们能找个借口手动编写一个 future 吗？
- 背景阅读
  - 关于关联类型的内容

来自 @sticnarf:
> 由于项目 5 只有大纲，我大多是根据自己的想法编写代码。希望这能在你编写正文时提供参考。 @brson
> 我将 concurrent_get/set 测试更改为使用异步。学生应该更改其 SledKvsEngine 和 KvStore 以适应具有新异步 API 的 KvsEngine trait。
> 引擎具有 ThreadPool 类型参数，构造函数具有 concurrency 参数（也许我们应该将其删除）。学生需要遵循此设计，以便测试能够正常工作。
> 我没有测试客户端。实现者可以自己选择客户端的 API 设计（除非我们制定出一个完美的设计，这样我们就可以直接向学生提供说明）。
-->
