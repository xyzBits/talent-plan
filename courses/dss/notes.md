# 项目概览与学习笔记

此笔记总结 `dss`（Distributed Systems in Rust）课程源码的每个子项目目的、关键文件、子项目间依赖关系，以及建议的学习顺序和学习要点。

---

## 1. 总体结构（工作区成员）
- `labcodec` — 编解码/Prost 相关的生成与示例。负责将 protobuf 定义生成 Rust 类型并提供序列化/反序列化支持。见 `build.rs`、`src/`。
- `labrpc` — 实验用的轻量 RPC 层；用 channel 模拟 socket，便于在不稳定网络中测试。包含 RPC 服务宏和示例。见 `src/`、`benches/`、`examples/`。
- `linearizability` — 提供线性化/模型测试、工具与测试数据（`test_data/`）。用于验证键值系统的线性化性质。
- `raft` — Raft 共识算法实验及基于 Raft 的 `kvraft`（lab3）。核心实现位于 `src/raft`，并使用 `src/proto` 中的 protobuf 定义。
- `percolator` — Percolator 分布式事务模型的实验实现（Timestamp Oracle、存储/锁等）。使用 protobuf 与 labrpc/labcodec。

工作区成员定义在根 `Cargo.toml` 的 `workspace.members`。

---

## 2. 每个子项目详细说明

### `labcodec`
- 目的：封装 protobuf（prost）代码生成与示例，提供用于 RPC/持久化的编码和解码工具。
- 关键文件：`build.rs`（调用 prost-build）、`src/`（生成/手写的辅助代码）、`demonstration/`（帮助理解过程宏的展开文件）。
- 主要依赖：`prost`、`prost-build`。
- 作用域：被 `labrpc`、`raft`、`percolator` 等本地 crate 以路径依赖引用，用于消息编解码与持久化格式。

### `labrpc`
- 目的：为实验提供一个可控的 RPC 框架，使用通道来模拟网络（可注入延迟、丢包等）。实现 RPC 服务与客户端 trait 的生成与测试工具。
- 关键文件：`src/lib.rs`、`src/server.rs`、`examples/echo.rs`、`benches/rpc.rs`。
- 主要依赖：`futures`（带 thread-pool 特性）、`async-trait`、`prost`、`rand`。
- 作用域：被 `raft`、`percolator` 等通过路径依赖引用，用于发送 Raft RPC / Percolator RPC。

### `linearizability`
- 目的：提供验证线性化一致性的工具、模型与测试数据。用于对 `kvraft` 的实现做一致性验证。
- 关键文件：`src/model.rs`、`src/models.rs`、`test_data/` 等。
- 主要依赖：仅在 dev 时引用 `regex`、`lazy_static` 等，用于测试/解析。
- 作用域：`raft`/`kvraft` 的测试与验证依赖它。

### `raft`
- 目的：实现 Raft 共识算法（lab2），并基于此构建容错键值服务 KvRaft（lab3）。涵盖 leader election、log replication、持久化、快照等实验内容。
- 关键文件：`src/raft/mod.rs`（实现）、`src/proto/raft.proto`、`src/proto/kvraft.proto`、`src/kvraft/server.rs`、`src/kvraft/client.rs`。
- 主要依赖（见 `raft/Cargo.toml`）：`labcodec`、`labrpc`、`linearizability`、`futures`、`prost`、`rand`、`log` 等。
- 作用域：是本仓库中最核心的系统实验，既依赖 `labrpc`/`labcodec`，又与 `linearizability` 配合用于测试。

### `percolator`
- 目的：实现 Percolator 分布式事务模型（TSO + 多列存储 + 事务协议），用于理解分布式事务与锁/冲突检测的实现细节。
- 关键文件：`src/`（TSO、MemoryStorage、客户端与事务实现）、`proto/`（消息定义）、`build.rs`（生成 prost 文件）。
- 主要依赖（见 `percolator/Cargo.toml`）：`labrpc`、`labcodec`、`prost`、`futures`。
- 作用域：侧重事务协议，与 `raft` 的关注点不同，但共享 `labrpc`/`labcodec` 基础设施。

---

## 3. 子项目之间的依赖关系（高层）
- `labcodec` 是底层编码/生成库。多数需要 protobuf 的子项目通过 `labcodec` 来生成/编码消息。
- `labrpc` 构建在 `labcodec` 之上（`labrpc` 的 `Cargo.toml` 指向 `labcodec`），为上层（`raft`、`percolator`）提供 RPC 能力。
- `raft` 依赖 `labrpc` 与 `labcodec`，并使用 `linearizability` 提供的工具进行一致性验证。
- `percolator` 依赖 `labrpc` 与 `labcodec`，但并不直接依赖 `raft`（两者关注不同的分布式问题）。
- `linearizability` 主要作为测试/验证工具用于 `raft/kvraft`。

依赖图（简化）：
labcodec <- labrpc <- { raft, percolator }
linearizability -> raft (测试/验证方向)

---

## 4. 推荐学习/实现顺序（逐步）
下面给出针对想从零到能实现 lab2/lab3 的学习顺序与要点。

1. 基础准备（先修）
   - 熟悉 Rust（所有示例都用 Rust 2018）。
   - 熟悉 protobuf（.proto 文件、prost、prost-build）。
   - 熟悉异步/并发基础（线程、channel、`futures` 基本概念）。

2. 阅读 `labcodec` 与 `labrpc`（先读基础设施）
   - 为什么先学：它们是上层实验（Raft/Percolator）的通信与消息基础；理解它们能帮助定位 RPC/编码错误。
   - 实践：查看 `labcodec/build.rs` 如何调用 `prost-build`，查看 `labrpc` 的 RPC 服务定义示例（`examples/echo.rs`）。

3. 理解 `linearizability`（测试思路）
   - 理解线性化的定义与如何用模型/测试数据验证客户端 API 的线性化。
   - 这会帮助你在实现 `kvraft` 时设计客户端请求的唯一标识与重复请求处理。

4. 实战：`raft`（lab2）
   - 按 2A → 2B → 2C 的顺序实现：先选举与心跳（2A），再日志复制（2B），最后持久化与快照（2C）。
   - 在实现过程中反复运行 `make test_2*`，并利用 `labrpc` 的不可靠网络测试进行验证。
   - 重点：角色转换（follower/candidate/leader）、日志一致性、commit 逻辑、持久化边界、并发与锁的划分。

5. 基于 Raft 的键值服务：`kvraft`（lab3）
   - 实现客户端 `Clerk` 的重试逻辑、 de-dup（请求唯一标识）、以及将操作通过 Raft 日志顺序化。
   - 实现快照/裁剪以控制 Raft 状态大小（3B）。
   - 使用 `linearizability` 测试验证实现的正确性。

6. 并行/进阶：`percolator`
   - 在理解分布式一致性与 Raft 后，学习 Percolator 的事务模型（TSO、两阶段提交/乐观并发控制、锁、冲突检测）。
   - 注意：Percolator 项目更多偏向事务协议与存储模型，对理解分布式事务很有帮助，但并非实现 Raft 的前提。

---

## 5. 学习与开发建议（实践提示）
- 先确保基础设施（`labcodec`、`labrpc`）的用法清楚，再动手实现 Raft 的细节。
- 在实现过程中大量运行单元测试，并使用 `labrpc` 的不可靠网络选项来复现分布式异常场景。
- 小步提交：先实现 2A（选举）并通过对应测试，再逐步向 2B/2C 推进。
- 注重日志与 debug：通过 `log`/`env_logger` 打印状态转移、RPC 收发与重要事件，有助于定位竞态与逻辑错误。
- 反复使用 `linearizability` 工具来验证 `kvraft` 的正确性，尤其是重复请求处理、线性化序问题。

---

## 6. 文件位置索引（便捷链接）
- 根 README（英文）： [README.md](README.md)
- 根 README（中文翻译）： [README_zh.md](README_zh.md)
- Raft 英文 README： [raft/README.md](raft/README.md)
- Raft 中文 README： [raft/README_zh.md](raft/README_zh.md)
- Percolator 英文 README： [percolator/README.md](percolator/README.md)
- Percolator 中文 README： [percolator/README_zh.md](percolator/README_zh.md)
- labcodec 演示（中文）： [labcodec/demonstration/README_zh.md](labcodec/demonstration/README_zh.md)

---

## 7. 我已检查的证据（关键依赖来自 `Cargo.toml`）
- 根 `Cargo.toml` 工作区成员包含：`labcodec`, `labrpc`, `linearizability`, `raft`, `percolator`。
- `raft/Cargo.toml` 明确依赖：`labcodec`、`labrpc`、`linearizability`。
- `percolator/Cargo.toml` 依赖：`labrpc`、`labcodec`。
- `labrpc/Cargo.toml` 依赖：`labcodec`。
- `labcodec/Cargo.toml` 使用 `prost` 与 `prost-build` 来生成 protobuf 代码。

---

## 8. 可选后续工作（我可以帮你做）
- 为 `labrpc`、`labcodec`、`linearizability` 的 README 也生成中文翻译（已为三处创建翻译；我可以继续）。
- 为每个子项目生成依赖图（DOT / 可视化图片）。
- 帮你创建一个 git commit 并推送（需要你授权/确认分支与提交信息）。

---

如果你希望我把这份 `notes.md` 进一步扩展为带有图表的版本（例如依赖关系图），或者自动将这些中文 README 都添加到一个目录下的索引页面，请告诉我下一步。