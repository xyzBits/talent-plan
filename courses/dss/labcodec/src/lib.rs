//! A thin wrapper of [prost](https://docs.rs/prost/0.6.1/prost/)
//! 这是一个对 prost 库（Rust 的 Protocol Buffers 实现）的轻量级封装模块。

/// A labcodec message.
/// 定义当前库通用的 Message 特征（Trait）。
/// 要求：所有实现此特征的类型，必须同时满足 `prost::Message`（基本 Protobuf 功能）和 `Default`（支持默认值）。
pub trait Message: prost::Message + Default {}

/// 覆盖实现（Blanket Implementation）：
/// 这是一行非常强大的 Rust 魔法。它表示：只要任何类型 T 满足了 `prost::Message + Default`，
/// 编译器就会自动通过这行代码，让它也实现我们定义的 `labcodec::Message`。
/// 这样用户就不需要手动为每个生成的 Protobuf 结构体写 `impl Message for X` 了。
/// 这个 Message 必须是我定义在当前  crate的 
impl<T: prost::Message + Default> Message for T {}

/// A message encoding error.
/// 类型别名：将 prost 的编码错误类型重新导出。
/// 作用：解耦。调用者不需要引入 prost crate，直接用 labcodec::EncodeError 即可。
pub type EncodeError = prost::EncodeError;

/// A message decoding error.
/// 类型别名：将 prost 的解码错误类型重新导出。
pub type DecodeError = prost::DecodeError;

/// Encodes the message to a `Vec<u8>`.
/// 泛型函数：接受任何实现了 Message 特征的类型 M。
/// 参数 message: 要编码的消息引用。
/// 参数 buf: 输出缓冲区，编码后的字节会追加到这个 Vec 中。
pub fn encode<M: Message>(message: &M, buf: &mut Vec<u8>) -> Result<(), EncodeError> {
    // 性能优化关键点：
    // message.encoded_len() 预先计算消息编码后需要的字节数。
    // buf.reserve() 提前在堆内存中分配足够的空间。
    // 这避免了在写入数据时 Vec 发生多次扩容（Reallocation）和数据拷贝，显著提高性能。
    buf.reserve(message.encoded_len());

    // 调用 prost 底层的 encode 方法将数据写入 buf。
    // `?` 操作符：如果出错则直接返回 Err，成功则继续。
    message.encode(buf)?;

    // 返回 Ok(()) 表示操作成功（Unit 类型）。
    Ok(())
}

/// Decodes an message from the buffer.
/// 解码函数：从字节切片中恢复出消息结构体 M。
pub fn decode<M: Message>(buf: &[u8]) -> Result<M, DecodeError> {
    // 直接调用 M 类型（实现了 prost::Message）的 decode 方法。
    M::decode(buf)
}

#[cfg(test)] // 只有在运行 `cargo test` 时才编译以下模块
mod tests {
    // 定义一个名为 fixture 的子模块，用于模拟生成的代码
    mod fixture {
        // The generated rust file:
        // 说明：在真实的 Rust Protobuf 项目中，.proto 文件会被编译成 .rs 文件。
        // 这些文件通常位于 target/debug/build/.../out/ 目录下。
        // 下面的 include! 宏就是把生成好的代码直接“复制粘贴”到这里。

        // 这是一个宏，用于动态包含路径下的文件。
        // env!("OUT_DIR") 获取构建脚本 (build.rs) 指定的输出目录。
        // 这里假设在这个目录下有一个 fixture.rs 文件（模拟生成的 Protobuf 结构体）。
        include!(concat!(env!("OUT_DIR"), "/fixture.rs"));
    }

    // 引入父模块定义的 encode 和 decode 函数以便测试
    use super::{decode, encode};

    #[test] // 标记这是一个测试函数
    fn test_basic_encode_decode() {
        // 1. 创建一个测试消息实例
        // fixture::Msg 是由 include! 宏引入的生成的结构体
        let msg = fixture::Msg {
            // 设置字段 type。
            // `as _` 是 Rust 的类型推断转换，把枚举值转为对应的整数类型（proto 中 enum 其实是 i32）。
            // r#type 是因为 `type` 是 Rust 关键字，所以用 `r#` 进行原始标识符转义。
            r#type: fixture::msg::Type::Put as _,
            id: 42,
            name: "the answer".to_owned(), // 将字符串字面量转为 String
            // 创建一个二维数组 Vec<Vec<u8>>。
            // vec![7; 3] 生成 [7, 7, 7]，再重复 2 次。
            paylad: vec![vec![7; 3]; 2],
        };

        // 2. 准备一个空的缓冲区
        let mut buf = vec![];

        // 3. 执行编码：msg -> buf
        // unwrap() 用于在测试中处理 Result，如果报错直接 panic 导致测试失败。
        encode(&msg, &mut buf).unwrap();

        // 4. 执行解码：buf -> msg1
        // Rust 编译器会自动推断 msg1 的类型应该是 fixture::Msg
        let msg1 = decode(&buf).unwrap();

        // 5. 断言：验证解码后的对象和原始对象完全相等（需要结构体实现 PartialEq）
        assert_eq!(msg, msg1);
    }

    #[test]
    fn test_default() {
        // 测试 Protobuf 的默认行为：空字节流应该解码为默认值。
        let msg = fixture::Msg::default();

        // 传入空切片 &[] 进行解码
        let msg1 = decode(&[]).unwrap();

        // 验证空切片解码出来的对象是否等于 Default::default()
        // 这也验证了我们在最上面定义的 trait Message: Default 约束的必要性。
        assert_eq!(msg, msg1);
    }
}