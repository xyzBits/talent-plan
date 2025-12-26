```markdown
# Raft 实验

这是一个使用 Raft 共识算法构建的键/值存储系统的一系列实验。该实验基于著名的 MIT 6.824 课程中的 [lab2:raft][6824lab2] 和 [lab3:kvraft][6824lab3]，并用 Rust 重写。实验中的许多文字说明也受该课程材料的影响。

在这些实验中，你将先在 lab2 中实现 Raft 共识算法，然后在 lab3 中构建键/值服务。

Raft 是一种设计为易于理解的共识算法。你可以在 [Raft 官方网站][raftsite] 阅读关于 Raft 的资料，包括[扩展的 Raft 论文][raftpaper]、Raft 的交互式可视化以及其他资源，这些资料有助于完成本实验。

[6824lab2]:http://nil.csail.mit.edu/6.824/2018/labs/lab-raft.html
[6824lab3]:http://nil.csail.mit.edu/6.824/2018/labs/lab-kvraft.html
[6824]:http://nil.csail.mit.edu/6.824/2018/index.html
[raftsite]:https://raft.github.io/
[raftpaper]:https://raft.github.io/raft.pdf

## 快速开始

首先用 `git` 克隆本仓库以获取实验源码。

然后，确保安装了 `rustup`。为了简化操作，建议也安装 `make`。

现在可以运行 `make test_others` 来检查项目是否正常。你应该能看到所有测试通过。

（如果你在 Windows 上，可能需要想办法使用 `make`，或者在控制台中手动运行 Makefile 中的命令，或使用 Windows Subsystem for Linux）

## lab2：Raft

在本实验中你将实现 Raft 共识算法。此实验分为 2A、2B 和 2C 三个部分。

运行本实验所有测试：

```sh
make test_2
```

建议多次运行测试，确保不是偶然通过。

单独运行某个测试：

```sh
make cargo_test_<测试名>
```

### 代码结构

实验相关代码主要位于 `src/proto/mod.rs`、`src/proto/raft.proto` 和 `src/raft/mod.rs`。

`src/raft/mod.rs` 应包含你的 Raft 主实现。测试器（以及 lab3 中的键/值服务器）会调用此文件中的方法以使用你的 Raft 模块。

服务通过 `Raft::new` 创建 Raft 节点，然后通过 `Node::new` 启动节点。测试器会调用 `Node::get_state`、`Node::is_leader` 和 `Node::term` 来获取节点的当前任期和是否为 leader。

当服务器需要将命令追加到日志时会调用 `Node::start`。`Node::start` 应立即返回，不应等待日志追加完成。`Raft::new` 中会传入一个 `apply_ch` 通道，对于每个新提交的日志条目，你应向该通道发送一个 `ApplyMsg`。

你的实现应使用提供的 `labrpc` crate 交换 RPC。`labrpc` 在内部用通道模拟套接字，便于在各种网络条件下测试。RPC 的定义放在 `src/proto/mod.rs`，并应在 `impl RaftService for Node` 中实现 RPC 服务端。`Raft::new` 会接收一组 RPC 客户端（`peers`），用于向其他节点发送 RPC。

### Part 2A

本部分需实现 leader 选举和心跳（只发送不包含日志条目的 `AppendEntries` RPC）。目标是让单个 leader 被选出，在没有故障时保持为 leader，当旧 leader 故障或与之通信丢失时能产生新 leader。

运行本部分所有测试：

```sh
make test_2a
```

提示：

- 向 `Raft` 结构添加所需状态字段。
- `request_vote` RPC 已定义，你需填写 `RequestVoteArgs` 和 `RequestVoteReply` 结构。实验使用 `labcodec` 来对 RPC 消息进行编解码，内部基于 `prost`。参阅 [prost][prost] 了解如何用 `#[derive(Message)]` 和 `#[prost(...)]` 定义消息结构。
- 你需要自己定义 `append_entries` RPC。`labrpc` 使用 `labrpc::service!` 宏从定义中生成服务端/客户端 trait。可参考 `labrpc/examples/echo.rs`。
- 实验大量使用 `futures` crate（如通道和 `Future` 特性）。参阅 [futures][futures]。
- 需要在周期性或延迟后触发操作。可以使用线程和 `std::thread::sleep`，也可以使用 `futures_timer::Delay` 等工具。
- 确保不同节点的选举超时不会总是同时触发，可用 `rand` 生成随机数。
- 测试器限制每对发送者-接收者的 RPC 频率为每秒 10 次，请不要频繁无间隔地发送 RPC。
- 选举应在旧 leader 故障后 5 秒内完成（如果多数节点仍可通信）。但超时不应过短（最好大于论文中 150~300 ms）。
- 在 Rust 中我们对数据上锁而非对代码上锁，请谨慎设计锁的范围。
- 可用 `log` 和 `env_logger` 打印调试信息，示例：`LOG_LEVEL=labs6824=debug make test_2a`。

[prost]:https://github.com/danburkert/prost
[futures]:https://docs.rs/futures/0.3/futures/index.html

### Part 2B

本部分实现日志复制：完成 `Node::start`，填充 `append_entries` RPC 的字段并发送它们，leader 需要推进 `commit_index`。

运行本部分所有测试：

```sh
make test_2b
```

提示：

- 注意选举限制，参见 Raft 论文第 5.4.1 节。
- 每个服务器应按照正确顺序通过写入 `apply_ch` 来提交新条目。
- 优化 RPC 次数以减少无需的调用。
- 可能需要编写等待某些事件发生的代码，可以使用通道阻塞等待。

### Part 2C

本部分实现持久化：将持久化状态保存到 `Persister`（在 `Raft::persist` 和 `Raft::restore` 中使用 `labcodec`），并在 `Raft::new` 中调用 `Raft::restore`。

运行本部分所有测试：

```sh
make test_2c
```

提示：

- 使用 `labcodec` 编解码持久数据。
- 该部分涉及在服务器故障和网络丢包情况下的多项挑战性测试，请仔细检查实现。
- 为通过某些“不可靠”测试，你可能需要实现 follower 在回退 `nextIndex` 时一次回退多于一个条目的优化。

[optimize-hint]:http://nil.csail.mit.edu/6.824/2018/notes/l-raft2.txt

## lab3：KvRaft

本实验使用 lab2 的 Raft 模块构建容错的键值存储服务，分为 3A 和 3B 两部分。

运行本实验所有测试：

```sh
make test_3
```

### 代码结构

实验代码主要在 `src/proto/mod.rs`、`src/proto/kvraft.proto`、`src/kvraft/server.rs` 和 `src/kvraft/client.rs`，另外还需修改 lab2 中你修改过的文件。

### Part 3A

本部分先实现一个在无丢包、无服务器故障时工作的方案，要求 `get(...)` 和 `put_append(...)` 保持线性化（linearizable）。

实现要点：

- 客户端在 `src/kvraft/client.rs` 发送 RPC
- 在 `KvServer` 的 RPC 处理器中通过 `raft::Node::start` 将客户端操作放入 Raft 日志
- 当 Raft 日志提交后执行操作并回复 RPC

通过基础测试：

```sh
make cargo_test_basic_3a
```

提示：

- 通过 `apply_ch` 接收 Raft 的提交消息。
- 如果 leader 在请求提交前失去领导权，客户端应重试直到找到新的 leader。
- `Clerk` 客户端应记录上次的 leader 优先尝试。
- 未达到多数或数据不够新的情况下，服务器不应完成 `get` RPC；可将 `get` 放入日志或实现论文第 8 节的读优化。
- 需处理重复客户端请求，保证每个操作只执行一次。

### Part 3B

本部分实现快照（snapshot）机制以节省 Raft 状态空间：kvserver 周期性保存快照并告知 Raft 丢弃快照前的日志条目。重启时，服务器先安装快照再回放之后的日志条目。

运行本部分所有测试：

```sh
make test_3b
```

提示：

- 可为 Raft 添加方法让 kvserver 管理日志裁剪与快照。
- 测试时可以将 `maxraftstate` 设置为 `Some(1)` 来测试裁剪行为。
- 将快照保存在 `raft::Persister`，并从中恢复。
- 在 `install_snapshot` RPC 中发送整个快照即可。

```
```