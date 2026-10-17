# qq-cli

文档编号：README-001。状态：Active。更新：2026-10-17。

让用户继续使用官方 QQ 窗口，同时让本地 Agent 通过命令行在后台调用同一会话。

qq-cli 使用 Rust 开发，计划作为已有 NapCat OneBot HTTP 实例的命令行客户端，首版提供状态、自身账号、
指定私聊历史、消息读取和单次文本发送，默认输出 JSON。技术方案见 [CLI 首版技术方案](docs/cli-design.md)。

目前已完成 M0：Rust/Cargo 工程与离线命令骨架。帮助、版本和参数校验可用；所有业务命令均返回
JSON 错误 NOT_IMPLEMENTED、outcome=not_attempted、退出码 6，不读取配置或消息输入、不连接后端。

```powershell
cargo run --offline --locked -- --help
cargo run --offline --locked -- message send --help
```

工具链固定 Rust 1.98.1；首次构建前需安装该工具链并执行 cargo fetch --locked 缓存依赖。
构建及离线检查见 [贡献指南](CONTRIBUTING.md)。CLI 的后端通信、消息读取和发送尚未实现。

原版 NapCat v4.18.38 Framework 已通过实验账号的有界消息记录对照及短时焦点观察；
实时事件、长期并行和恢复尚未验证，实验结果不代表 CLI 已实现或适用于所有 QQ 版本。
不修改 NapCat、不 fork、不开发加载器；CLI 不负责启动、登录或关闭 QQ。
适用许可与账号风险仍有未解决事项，不代表消除了注入、协议或封号风险。

开发与上下文入口见 CONTRIB-001、GOV-001、DESIGN-001；按文档编号在仓库检索。
