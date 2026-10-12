# 编码与构建器

**WeChat**

---

**编码与构建器：用 Rust 创建 HL7 消息**

解析总是最受关注的。但生产环境中的 HL7 系统也需要创建消息——ACK 响应、转换后的输出、新医嘱。

VynaHL7 编码和构建器基准测试：

将已解析的 Message 编码回 HL7 字符串（单线程）：5.3 µs（ADT），15.2 µs（ORU）。吞吐量 47 MiB/s——比解析快 3-4 倍，因为结构已经确定。

转义序列处理：编码纯文本 177 ns，重度转义（40% 特殊字符）339 ns。解码纯文本？27 ns——它只是扫描反斜杠，然后发现没有。

使用构建器模式从零创建消息：一条含 13 个字段的 ADT^A01 仅需 9.7 µs，与解析同等大小的消息（315B ADT 解析需 13.6 µs）相当，比解析完整的 959B ADT（41 µs，包含更多字段）更快。

Rust 的构建器模式使用所有权转移（self，而非 &mut self）。编译器将方法链优化为就地修改——没有中间拷贝：

```rust
let msg = AdtA01Builder::new(Version::V2_5)
    .patient_id("MRN12345")
    .patient_name("DOE", "JOHN")
    .build()?;
```

实用建议：创建新消息时，构建器优于解析。ACK 和新医嘱用构建器，消息转换用解析+修改+编码。

完整技术分析：
https://www.flostel.com/posts/vyna-hl7-technical-deep-dive-part-7-of-11

#HL7 #Rust #医疗信息化 #性能优化 #构建器模式
