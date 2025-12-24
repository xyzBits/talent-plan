use serde::{Deserialize, Serialize};

// 详细中文注释（补充）：
// 1. 协议设计：`Request` 与 `*Response` 枚举定义了客户端与服务器之间的 JSON-RPC 式消息格式（但没有使用完整的 JSON-RPC 标准），
//    通过 `serde` 自动序列化/反序列化为 JSON。使用枚举可以在单个字段中保存不同的消息类型，便于扩展与解析。
// 2. 向后兼容性与版本：
//    - 在设计协议时应注意兼容性（新增变体或字段时要考虑旧客户端/服务器如何处理）。当前简单实现假定客户端与服务器版本一致。
// 3. 错误表达：
//    - 对于 `GetResponse::Err(String)` 等变体，服务器会把错误信息打包成字符串返回；客户端收到后将其映射为 `KvsError::StringError`。
// 4. 对 Rust 新手的建议：
//    - 使用 `serde` 时，枚举的序列化形式是可控的（tagged、untagged 等），默认行为在本仓库里足够直观，但如果需要与其他语言互通，可显式指定序列化策略。

#[derive(Debug, Serialize, Deserialize)]
pub enum Request {
    Get { key: String },
    Set { key: String, value: String },
    Remove { key: String },
}

#[derive(Debug, Serialize, Deserialize)]
pub enum GetResponse {
    Ok(Option<String>),
    Err(String),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum SetResponse {
    Ok(()),
    Err(String),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum RemoveResponse {
    Ok(()),
    Err(String),
}
