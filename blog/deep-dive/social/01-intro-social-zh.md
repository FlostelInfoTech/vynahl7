# VynaHL7 介绍

**WeChat**

---

**VynaHL7：用 Rust 构建的 HL7 v2 医疗消息处理库**

HL7 v2 是一个有30年历史的协议，却仍然是医疗信息集成的核心——每年医院、实验室和药房之间交换数十亿条 HL7 v2 消息。

VynaHL7 是一个用 Rust 编写的 HL7 v2 全生命周期处理库：

- 单线程内存解析速度：17,000-47,000 条消息/秒
- 支持 HL7 v2.3 至 v2.8 的 schema 验证
- 流式构建器（Fluent Builder）创建消息
- 12 个 FHIR R4 转换器，支持现代互操作性
- MLLP 和 HTTP 传输层，支持 TLS/mTLS
- WebAssembly 支持，可在浏览器中运行

无垃圾回收器。无反射机制。基于零成本抽象的确定性性能表现。

我们正在发布一个 11 篇的技术深度剖析系列，涵盖解析、内存、验证、字段访问、FHIR 转换和网络传输的性能测量。每一个数据都可以通过 `cargo bench` 命令复现。

第 1 篇从这里开始：
https://www.flostel.com/posts/vyna-hl7-technical-deep-dive-part-1-of-11

#HL7 #FHIR #医疗信息化 #Rust #互操作性 #HealthIT
