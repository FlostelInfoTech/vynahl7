# 完整流水线

**WeChat**

---

**完整流水线：从网络传输到 FHIR**

我们发布了 10 篇文章，对 VynaHL7 的每一层进行基准测试——从 10 ns 的数据类型验证到 137 µs 的 MLLP 网络往返。

以下是一个用 Rust 编写的 HL7 集成库的完整性能全景：

内存操作均为单线程。网络数据标注了客户端数量。

解析 ADT^A01：41 µs（24,200/秒，单线程）
Schema 验证：+6 µs
提取 10 个临床字段：+7 µs
编码回 HL7 字符串：5 µs（47 MiB/s）
转换为 FHIR Patient：4.3 µs
序列化为 JSON：650 ns
MLLP 网络往返：137 µs p50（单客户端）
MLLP 持续吞吐量：7,200 msg/s 单客户端 = 每天 6.24 亿条消息

接口引擎模拟（解析 + 路由 + ACK）：单线程内存处理每条消息 19.7 µs，即单核 50,000 msg/s——不含网络 I/O。

每一个数据都有一份四段式基准测试报告（目的、方法论、配置、结果）。每一个数据都可以用一条命令复现。

一致性发现：Rust 的所有权模型、零成本抽象和跨 crate 编译时 LTO 使性能不再是医疗信息集成的瓶颈。真正的瓶颈在于你的网络、数据库或业务逻辑——而非 HL7 库本身。

VynaHL7 是开源项目。自己运行基准测试：
```
git clone https://github.com/flostelinfotech/vyna-hl7.git
cd vyna-hl7 && bash bench-results/run_all.sh
```

完整 10 篇系列：
https://www.flostel.com/posts/vyna-hl7-technical-deep-dive-part-11-of-11

#HL7 #FHIR #医疗信息化 #Rust #开源 #性能优化 #HealthIT #互操作性
