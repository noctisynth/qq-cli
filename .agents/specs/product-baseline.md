# CLI 技术基线与系统边界

文档编号：SPEC-001。状态：Active。更新：2026-10-24。
需求：RFC-0002/0003/0004。代码导航：DESIGN-002。公共接口：PLAN-001。

## 组件边界

Rust 单 binary crate 提供 CLI、类型化 OneBot HTTP/WS 适配、领域用例与持久状态。
NapCat Framework 和官方 QQ 是外部运行依赖；Rust 业务层不直接接 QQ Electron/Node ABI。
平台运行层负责启动装配、文件/进程/连接身份与受保护凭据；内部最小适配的边界见 SPEC-003/008。
CLI 命令由 app 组合领域、存储、传输和平台能力，不让生命周期内核构造 CLI 输出。

## 数据与信任边界

实例选择 → 签名运行记录和所有权恢复 → 受检连接 → 账号/版本核验 → 领域操作。
不能将 URL/token、登记 ready、PID 或 API 成功单独视为实例身份和兼容证明。
公共 JSON/NDJSON 与 Schema 同源，详见 SPEC-007；普通命令不暴露 raw action 或管理秘密。
持久签名引用绑定实例、账号、运行代次及目标；旧引用不随切号/换代迁移。

## 平台和版本

Windows 发布与功能证据由 TASK-002 维护；原生能力边界在 native/windows。
macOS 仅 arm64，Framework 原型与 Rust 产品实现分开；技术方案 SPEC-008，验收 TASK-006。
已验证版本不能自动推广到未来 QQ/NapCat、其它架构或系统版本；兼容键和支持判定见 SPEC-006。
工程编译器版本固定于 rust-toolchain.toml/Cargo.toml，不代表 QQ 内部运行时版本。
