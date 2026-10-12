# HL7 v2 到 FHIR R4 转换

**WeChat**

---

**从 HL7 v2 到 FHIR R4：微秒级转换**

大多数医院系统使用 HL7 v2，而现代 API 需要 FHIR R4。在两者之间架桥是医疗信息集成的日常工作。

VynaHL7 包含 12 个 FHIR R4 转换器。它们有多快？

| 转换器 | 耗时 |
|--------|------|
| Condition（DG1 → FHIR） | 627 ns |
| AllergyIntolerance（AL1 → FHIR） | 869 ns |
| Practitioner（PV1 → FHIR） | 1.94 µs |
| Patient（PID → FHIR） | 4.29 µs |
| 4 个 Observation（OBX → FHIR） | 13.1 µs |

完整流水线——从原始 HL7 字符串到 JSON 序列化的 FHIR 资源（单线程）：
- ORU → 全部资源 + JSON：63 µs（16,000 次转换/秒）
- ADT → 全部资源 + JSON：29 µs（34,000 次转换/秒）

JSON 序列化本身？一个 Patient 资源仅需 650 ns。Rust 的 serde 在编译时生成专用的序列化代码——无反射、无运行时类型检查。每次字段写入都是直接方法调用。

处理一天的检验结果（100,000 条 ORU 消息）仅需 6.3 秒的单线程 CPU 时间。

完整深度剖析，含流水线分解：
https://www.flostel.com/posts/vyna-hl7-technical-deep-dive-part-8-of-11

#FHIR #HL7 #医疗信息化 #互操作性 #Rust #HealthIT
