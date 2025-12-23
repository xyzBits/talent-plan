use serde::{Deserialize, Serialize};

/// 定义网络协议支持的请求类型
#[derive(Debug, Serialize, Deserialize)]
pub enum Request {
    /// 获取给定键的值
    Get { key: String },
    /// 设置给定键的值
    Set { key: String, value: String },
    /// 移除给定的键
    Remove { key: String },
}

/// Get 请求的响应结果
#[derive(Debug, Serialize, Deserialize)]
pub enum GetResponse {
    /// 成功，包含可选的值
    Ok(Option<String>),
    /// 失败，包含错误消息字符串
    Err(String),
}

/// Set 请求的响应结果
#[derive(Debug, Serialize, Deserialize)]
pub enum SetResponse {
    /// 成功
    Ok(()),
    /// 失败，包含错误消息字符串
    Err(String),
}

/// Remove 请求的响应结果
#[derive(Debug, Serialize, Deserialize)]
pub enum RemoveResponse {
    /// 成功
    Ok(()),
    /// 失败，包含错误消息字符串
    Err(String),
}
