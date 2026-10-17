# 面向 coding agent 的 Rust CLI

文档编号：RFC-0002。状态：Accepted，扩展契约的后端与依赖待验证。更新：2026-10-17。
关联：SPEC-004、PLAN-001、TASK-002、CHECK-008。用户明确 Rust 和完整交付。

## 决策

单 Cargo binary crate、edition 2024、Cargo.lock；Cargo.toml 唯一版本来源，Semifold rust resolver。
Rust 1.98.1、clap derive、serde/serde_json、reqwest blocking、sha2 的工程基础已离线核验。
类型化 OneBot HTTP 适配器与正向 WS 适配器连接现有 NapCat，不接入 QQ ABI 或 WebUI。
普通 Rust 子命令按任务组织，不复制 Lark 的特殊前缀，不暴露任意 raw action。
命令注册表共享 schema、帮助和任务指引，不为不同 Agent 维护不同 CLI 语义。

采用 SPEC-004 完整契约：多账号、发现、私聊/群聊、历史、附件、事件、恢复及 Agent 指引。
替代此前六 action、私聊文本及分期交付的范围限制；历史验收记录保留，骨架不代表产品完成。
输出 JSON/NDJSON、显式 profile/会话、预算有界、持久防重和未知结果保守处理。
同账号 profile 别名共享防重域；不把本地去重说成上游 exactly-once。

## 理由与取舍

Rust 为用户决定，原生进程不涉及 QQ Electron/Node ABI，以 CLI 为 Agent 宿主提供共同契约。
借鉴 Lark 的能力发现和任务指引，选择类型化任务操作以验证身份/权限/副作用。
不采用 UI 自动化、独立机器人身份或必要常驻服务；有界 watch 不隐式创建后台任务。
普通命令保持同步 HTTP，事件循环独立；WS、文件锁、信号和持久化依赖需验证 Windows、许可及锁定版本。
通用 SKILL 包是指引载体，导出不等于安装成功或宿主已兼容，分别验收。

## 实施前待收敛

- 固定候选 action/消息段、私聊/群历史游标和 ID 生命周期、媒体路径、WS 身份及自发上报条件。
- 附件多步写状态机及文件传输/消息接受不同语义；后端不足登记阻塞再解决。
- 完整注册表、默认/硬预算、游标/引用完整性、并发状态、配置及升级兼容。
- Windows 依赖验证、许可、明确授权的真实回归及 Codex/OpenCode 任务闭环。

全部纳入 TASK-002；待验证不能写成实现，本轮不 fork 或新增真实消息操作。
