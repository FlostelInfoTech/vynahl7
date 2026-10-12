# HTTP 传输与协议对比

**WeChat**

---

**HTTP vs MLLP：医疗集成该选哪种传输协议？**

院内用 MLLP，跨机构用 HTTP。VynaHL7 两者都支持。它们的表现如何？

相同消息、相同硬件下的正面对比：

| 指标 | MLLP | HTTP | 优势方 |
|------|------|------|--------|
| 吞吐量（单客户端） | 6,000 msg/s | 3,320 msg/s | MLLP 快 1.8x |
| p50 延迟 | 0.137 ms | 0.275 ms | MLLP 快 2.0x |
| 8 客户端吞吐量 | 49,900 msg/s | 24,100 msg/s | MLLP 快 2.1x |
| 日持续吞吐量 | 6.24 亿条 | 3 亿条 | MLLP 快 2.1x |

MLLP 全面快约 2 倍。这个差距是稳定的——不会随并发数增减而变化。

HTTP 的开销来自哪里？不是 Rust 框架。Axum 封装 Hyper，Hyper 封装 Tokio——两层抽象在编译时被消除（零成本抽象）。2 倍的差异是协议开销：HTTP 头、状态行、内容协商。MLLP 仅添加 3 字节帧头，HTTP 则增加约 200 字节。

但关键在于：即使 HTTP 单客户端 3,320 msg/s（基于回环 TCP 测试），也已超过大多数医疗机构的需求。应根据网络需求选择协议，而非性能。

院内用 MLLP，机构边界用 HTTP。两者均支持 TLS/mTLS。

完整对比及扩展数据：
https://www.flostel.com/posts/vyna-hl7-technical-deep-dive-part-10-of-11

#HL7 #MLLP #HTTP #医疗信息化 #Rust #互操作性
