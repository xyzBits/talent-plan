use serde::{Deserialize, Serialize};

/// 客户端请求枚举，定义了支持的操作
#[derive(Debug, Serialize, Deserialize)]
pub enum Request {
    /// 获取键对应的值
    Get { key: String },
    /// 设置键值对
    Set { key: String, value: String },
    /// 移除键
    Remove { key: String },
}

/// 服务器响应枚举，定义了操作的处理结果
#[derive(Debug, Serialize, Deserialize)]
pub enum Response {
    /// Get 操作的响应，返回可选的字符串值
    Get(Option<String>),
    /// Set 操作成功响应
    Set,
    /// Remove 操作成功响应
    Remove,
    /// 发生错误时的响应，包含错误信息字符串
    Err(String),
}
