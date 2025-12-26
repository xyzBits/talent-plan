# Rust 中的分布式系统

这是一个关于 [Rust] 中分布式系统的培训课程。

涵盖的主题包括：

- [Raft 共识算法]（包括使用 Raft 实现的容错键值存储服务）
- [Percolator 事务模型]

完成本课程后，您将具备用 Rust 实现一个具有事务支持和容错性的基本键值存储服务的知识。

**重要提示：本课程处于 alpha（早期）状态**
它可能包含错误。非常欢迎反馈。如果您遇到任何问题，请[提交 issue]。同时也鼓励您自行修复问题并提交 pull request。

## 本课程的目标

本课程旨在教导对分布式系统感兴趣的 Rust 程序员，了解如何构建可靠的分布式系统以及如何实现分布式事务。

## 适合对象

本课程适用于有经验的 Rust 程序员，熟悉 Rust 语言。如果您尚不熟悉 Rust，可以先学习我们的 [rust] 课程。

## PingCAP 相关说明

本课程与 [Deep Dive TiKV] 相结合，旨在帮助程序员能够有意义地为 [TiKV] 做贡献。它特别面向中国的 Rust 社区，使用的语言尽量简单，方便只懂一点英语的读者理解。如果您发现文中语言难以理解，请[提交 issue]。

## 许可

[CC-BY 4.0](https://opendefinition.org/licenses/cc-by/)

<!-- 链接 -->
[rust]: ../rust/README.md
[提交 issue]: https://github.com/pingcap/talent-plan/issues/
[Deep Dive TiKV]: https://tikv.github.io/deep-dive-tikv/overview/introduction.html
[TiKV]: https://github.com/tikv/tikv/
[Rust]: https://www.rust-lang.org/
[Raft 共识算法]: raft/README.md
[Percolator 事务模型]: percolator/README.md
