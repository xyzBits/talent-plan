# labrpc（实验 RPC 框架）

## 目标

`labrpc` 是一个用于课程实验的轻量级 RPC 层。它使用 Rust 通道来模拟网络连接（socket），便于在可控的测试环境下注入延迟、丢包和重排序等网络故障，从而测试分布式算法的鲁棒性。

## 主要功能
- 提供宏与工具以在本地生成 RPC 服务端与客户端 trait。 
- 用通道模拟网络，可配置为可靠或不可靠网络，用于复现分布式异常场景。
- 包含示例（`examples/`）与基准（`benches/`）用于演示和性能测试。

## 关键文件
- `src/lib.rs`：库主入口与公共 API。
- `src/server.rs`、`src/client.rs`：RPC 服务/客户端实现（实现细节请查看源代码）。
- `examples/echo.rs`：RPC 示例，用于学习如何定义与使用 RPC。
- `benches/rpc.rs`：RPC 的基准测试。

## 如何使用
1. 查看 `examples/echo.rs` 了解 RPC 服务定义与调用方式。
2. 在上层 crate（例如 `raft`、`percolator`）的 `.proto` 文件生成 Rust 类型后，使用 `labrpc::service!` 宏定义 RPC 接口并实现服务端。

运行示例或基准（在仓库根或直接在 `labrpc` 目录运行）：

```sh
# 运行示例（用 cargo run 指定示例路径）
cargo run --example echo --manifest-path labrpc/Cargo.toml

# 运行基准（需要启用基准工具）
cargo bench --manifest-path labrpc/Cargo.toml
```

## 调试与测试
- `labrpc` 与上层模块一起使用时，可通过调整模拟网络参数（延迟、丢包）来测试分布式算法在不可靠网络下的行为。
- 查看 `labrpc` 源代码以了解如何构建客户端列表（peers）并在节点间发送 RPC。
