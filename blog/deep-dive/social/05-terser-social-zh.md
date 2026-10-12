# Terser 与 CachedTerser

**WeChat**

---

**Terser 与 CachedTerser：大规模 HL7 字段访问**

解析完 HL7 消息后，你需要提取字段——患者ID、检验值、医嘱状态。

VynaHL7 提供两种方式：Terser（直接树遍历）和 CachedTerser（基于 HashMap 的缓存查找）。

对于重复访问，CachedTerser 快 1.67 倍——10 个路径各访问 5 次，4.72 µs vs 7.89 µs。

真实临床工作流的提取时间：
- 患者人口统计学信息（6 个字段）：1.58 µs
- 8 个 OBX 段的检验结果（32 个字段）：13.4 µs
- 医嘱信息（5 个字段）：1.31 µs

Rust 的优势：Terser<'a> 持有对 Message 的借用引用，带编译时生命周期保证。字段访问返回直接指向已解析数据的 &str 切片——零拷贝。借用检查器在编译时证明你提取的引用不会超过消息的生命周期，无需防御性克隆。

单线程下每次字段提取仅需 263 ns，可以在一台笔记本上每秒从 74,000 条检验消息中提取全部字段——仅为字段提取时间，不含解析时间。

完整基准测试数据及扩展性分析：
https://www.flostel.com/posts/vyna-hl7-technical-deep-dive-part-5-of-11

#HL7 #Rust #医疗信息化 #性能优化 #互操作性
