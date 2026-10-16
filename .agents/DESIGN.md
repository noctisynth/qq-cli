# qq-cli 工程设计索引

文档编号：DESIGN-001。状态：Active。更新：2026-10-16。

## 产品方向与当前状态

面向个人 QQ 的本地 Agent 命令行接入，保留官方桌面窗口，与用户共用同一会话，不抢占输入焦点。
不依赖 LiteLoaderQQNT，不创建独立官方机器人身份。当前仅初始化工程上下文和版本管理。
独立加载器能否复用原版 NapCat Framework 发布产物而无需 fork，是待验证问题。
不依赖 LiteLoader 不等于不使用 Framework 的内部接入逻辑；也不等于无 Hook 或注入。

## 文档注册表

| 编号 | 文件名 | 类型 | 责任 |
|---|---|---|---|
| README-001 | README.md | 介绍 | 产品目标与当前可用性 |
| CONTRIB-001 | CONTRIBUTING.md | 贡献指南 | 工程命令与协作入口 |
| GOV-001 | AGENTS.md | Active 约定 | 授权、事实、隐私与验证纪律 |
| DESIGN-001 | DESIGN.md | Active 索引 | 方向、阅读顺序与责任边界 |
| SPEC-001 | product-baseline.md | Active Spec | 已确定目标与非目标 |
| SPEC-002 | repository-bootstrap.md | Active Spec | Git、Semifold、上下文初始化 |
| FACT-001 | napcat-architecture.md | 源码观察 | Shell、桌面会话接入与未验证边界 |
| EVIDENCE-001 | evidence-register.md | 证据索引 | 公开来源和验证范围 |
| CHECK-001 | bootstrap-verification.md | 验证记录 | 本次初始化结果 |
| RFC-0000 | README.md | Active 流程 | 决策状态和接受条件 |
| RFC-0001 | 0001-session-loader.md | Draft RFC | 独立加载器、产物复用、fork 条件 |
| TODO-001 | TODO.md | Active 索引 | 实施清单入口 |
| TASK-001 | project.md | Active 清单 | 当前进度和后续验证差距 |

## 候选责任边界

| 层 | 预期职责 | 当前状态 |
|---|---|---|
| CLI | 命令、参数、结构化结果与错误 | 未设计、未实现 |
| 本地 API 适配 | 与 NapCat 接口通信、结果确认 | 待固定版本和契约 |
| 独立加载器 | 保留官方启动链，接入同一会话 | 候选方案，未实验 |
| 上游 NapCat | 内部消息能力与协议适配 | 未下载、未集成 |

上述职责划分用于研究，不冻结语言或组件数量。初始化不启动 QQ，不变更 QQ 安装。
