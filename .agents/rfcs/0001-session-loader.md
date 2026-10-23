# 无 LiteLoader 的桌面会话接入

文档编号：RFC-0001。状态：Superseded。更新：2026-10-24。
替代：产品范围见 RFC-0002，环境管理见 RFC-0003，Apple Silicon 扩展见 RFC-0008。

本提案最初要求研究保留官方 QQ 窗口、共享个人会话且不依赖 LiteLoader 的路径。
优先复用原版 Framework；缺少入口时先记录失败，再论证最小适配，不预设需要 fork。

该需求已纳入上述方案。Windows 与 macOS 的机制和证据分别维护，不能将 Shell 独立会话视为共享 GUI 的替代验收。
历史来源与原生加载器限制见 FACT-003/004；通用验证设计见 SPEC-003，macOS 技术路线见 SPEC-008。
历史实验中的一次性授权不成为后续启动、读取或发送许可。
