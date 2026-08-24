# 🚀 AGY Bridge (`ttagy`)

> 跨项目通用本地 AI / Antigravity CLI 守护服务与多语言客户端 SDK 基础设施

---

## 🌟 核心特性

- ⚡ **零冷启动延迟**：常驻守护服务 (`ttagyd`) 通过 Unix Domain Socket 维护会话池，调用延迟 $\le 2\text{ms}$。
- 🛡️ **沙箱与 Token 隔离**：强制独立临时沙箱与专属 `--log-file`，彻底消除 35k 目录树上下文泄漏与文件锁冲突。
- 🔄 **双模自愈与自动降级**：客户端优先直连 Daemon，未启动时透明回退至进程内沙箱 Worker。
- 📦 **多语言 SDK 支持**：提供原生 Rust (`ttagy-client`)、TypeScript (`@ttagy/client`) 与 Python (`ttagy_client`)。

---

## 🛠️ 模块索引

| 模块路径 | 语言 / 类别 | 描述 |
| :--- | :--- | :--- |
| `crates/ttagy-core` | Rust Core | 类型契约、沙箱隔离管理器、二进制探查、NDJSON 流式解析器 |
| `crates/ttagy-client` | Rust SDK | 异步流式客户端，支持 UDS IPC 优先与 In-Process Fallback |
| `crates/ttagyd` | Rust Binary | 本地常驻守护进程服务 |
| `packages/ttagy-client` | TypeScript SDK | 支持 Node.js、Electron 与 Web 运行时 |
| `python/ttagy_client` | Python SDK | 支持 asyncio 异步流式生成器 |
## 📄 开源许可证

本项目采用 **`BSD-3-Clause OR Apache-2.0`** 双开源许可证，与 TTZip Core 保持一致。详见 [LICENSE-BSD](./LICENSE-BSD) 与 [LICENSE-APACHE](./LICENSE-APACHE)。
