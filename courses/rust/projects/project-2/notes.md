## 1. BufWriter

    - 调用 file.write(&[byte]) 时，程序需要发起一个 system call，请求 OS 把数据写到硬盘
    - bufWriter.write()时，只是把数据 copy 到内存的缓冲区
    - 只有 bufWriter.flush()时，都会发起一次系统调用

## 2. serde_json::to_writer() 流式序列化
    - 将 Rust 结构体转换为 JSON
    - serde_json::to_string(&cmd) 会生成一个 string 对象
    - to_string 内存杀手，要在堆内存中分配空间来存放整个 JSON 字符串，如果对象有100MB，就要申请 100MB内存
    - to_writer 不需要生成中间的string，它一边序列化，一边把产生的字节直接喂给writer，这是一个流式的过程，几乎不会占用额外的堆内存

