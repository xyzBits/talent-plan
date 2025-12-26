````markdown
# Percolator 实验

## 什么是 Percolator

Percolator 是 Google 为在超大数据集上进行增量处理而设计的系统，同时提供带有 ACID 快照隔离语义的分布式事务协议。更多细节请参阅论文：[Large-scale Incremental Processing Using Distributed Transactions and Notifications](https://storage.googleapis.com/pub-tools-public-publication-data/pdf/36726.pdf)。

## 实验先决条件

开始本实验前，你需要：

1. 熟悉 Rust（也可参考我们的 Rust 培训课程）
2. 了解 protobuf 的工作原理
3. 具备基本的 RPC 知识
4. 具备分布式事务的基本概念

## 实验中的概念

### 服务器

本实验包含两类服务器：TSO 服务器（Timestamp Oracle）和存储服务器。

#### TSO 服务器

Percolator 依赖一个名为时间戳 Oracle（TSO）的服务。`TimestampOracle` 实现的 TSO 可以产生严格递增的时间戳，所有事务通过获取唯一的时间戳来指示执行顺序。

#### 存储服务器

Percolator 基于 Bigtable 提供多维排序映射。该实验通过 `MemoryStorage` 模拟 Bigtable，使用三列（由 `BTreeMap` 实现）来模拟 Bigtable 中的列：`Write`、`Data`、`Lock`。

存储服务器还需提供基础操作如 `read`、`write` 和 `erase` 来操作数据。

### 客户端

客户端会 `begin` 一个事务（包含一组操作，如 `get`、`set`），并调用 `commit` 提交事务。同时客户端可调用 `get_timestamp` 获得时间戳。

更多实现细节请参见论文。

## 自行实现

项目中留有诸如 “Your definitions here” 或 “Your code here” 的注释。你需根据论文自行实现。对结构体和 proto 的定义没有太多限制，可根据需要添加字段以实现功能。

## 测试你的实现

在本目录下直接运行：

```sh
make test_percolator
```

````