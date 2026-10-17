# 基于现有 OneBot HTTP 的 CLI

文档编号：RFC-0002。状态：Accepted direction，未实现。更新：2026-10-17。
关联：SPEC-004、PLAN-001、TASK-002。用户明确采用 Rust；具体依赖版本及工具链待实施验证。

## 推荐选择

CLI 为独立进程，只经六个已核对的 OneBot action 与已有实例交互，不接入 QQ ABI 或 WebUI 管理 API。
采用 Rust、单个 Cargo binary crate、edition 2024、Cargo.lock，构建原生 qq-cli。
依赖计划为 clap derive、serde/serde_json、reqwest blocking、sha2；测试优先内置测试框架和标准库 HTTP 假服务。
reqwest 关闭默认 features、仅开 blocking/json，显式关闭代理、重定向和自动重试；超时和响应体限制由客户端落实。
同步命令不引入应用层异步运行时或常驻服务；后续实时事件另行设计。
使用 clap 的 try_parse 路线把参数错误接入统一 JSON/退出码，不让默认错误输出绕过业务契约；help/version 保持文本。

理由：当前 JSON/消息段接口规模小，类型约束与平台无关进程通信足够；无需复制上游实现、打包 QQ 或维护原生模块。
用户选择 Rust 为决定性依据。原先 TypeScript/Node 建议被替换，现有 nodejs 版本解析器仅为初始化遗留。

## 备选及代价

Python 标准库已用于忽略目录内的实验探针，能完成 HTTP，但探针没有正式命令、错误、包装和防重契约，不能直接冒充产品。
Rust 原生可执行文件适合本地 CLI 分发；构建工具链与依赖维护成本由 Cargo 锁文件及 CI 检查承担。
不引入通用 Bot SDK，以减少额外隐式 action、自动重试和事件生命周期。

## 关键约束

输出 schemaVersion 固定为 1；只允许显式目标，发送前检查期望账号。
调用方提供请求键，本地记录处理重复命令与结果未知，不宣称上游支持 exactly-once。
不把实时事件塞入首版；收到消息的 MVP 路径是显式指定会话的有界历史查询。
先原创新 CLI 和合成测试，不从上游复制实现；许可、发布与账号风险问题不能由这项技术选择自动解决。

## 实施时收敛

进入 M0 时建立 Rust 工具链、Cargo 工程与锁文件，确定最低编译器版本；依赖组合必须经实际构建与测试验证。
建立 Cargo.toml 后核对 Semifold 对 Cargo 的解析配置，迁移版本来源并移除无用 package.json，不维护两套版本真值。
当前仅有技术方案授权，M0 尚未执行；独立打包、支持其他平台和公开发布留到后续。
