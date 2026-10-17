# qq-cli 工程设计索引

文档编号：DESIGN-001。状态：Active。更新：2026-10-17。

## 产品方向与当前状态

面向个人 QQ 的本地 Agent 命令行接入，保留官方桌面窗口，与用户共用同一会话，不抢占输入焦点。
不依赖 LiteLoaderQQNT，不创建独立官方机器人身份。当前完成初始化、审计、有界运行实验及 CLI M0，业务功能仍未实现。
NapCat v4.18.38 的 Framework 包已有原生加载入口，优先评估原版，不必立即开发加载器或 fork。
CHECK-005、CHECK-006 已有实验账号登录、基础 API、消息历史对照和短时焦点证据；长期共存、原始编译来源与适用许可仍有缺口。
不依赖 LiteLoader 不等于不使用 Framework 的内部接入逻辑；也不等于无 Hook 或注入。
FACT-004 补充维护者曾推荐原生入口，旧 Once/OneKey 才是明确弃用对象；现行指南缺席原因未明。
两种模式均有 Bypass 调用，不能保证账号安全；如要求无下线或封禁风险，当前候选不满足要求。
当前完成独立 CLI M0：Rust 工程、离线命令骨架、Semifold 迁移和验证，不修改 NapCat、不新增真实消息操作。
用户文档入口 PLAN-001；实施范围 SPEC-004；Rust 选型 RFC-0002；下一步任务 TASK-002。

## 文档注册表

| 编号 | 文件名 | 类型 | 责任 |
|---|---|---|---|
| README-001 | README.md | 介绍 | 产品目标与当前可用性 |
| CONTRIB-001 | CONTRIBUTING.md | 贡献指南 | 工程命令与协作入口 |
| GOV-001 | AGENTS.md | Active 约定 | 授权、事实、隐私与验证纪律 |
| DESIGN-001 | DESIGN.md | Active 索引 | 方向、阅读顺序与责任边界 |
| SPEC-001 | product-baseline.md | Active Spec | 已确定目标与非目标 |
| SPEC-002 | repository-bootstrap.md | Active Spec | Git、Semifold、上下文初始化 |
| SPEC-003 | framework-integration-plan.md | Active 验证计划 | 原版入口、生命周期及分阶段验收 |
| SPEC-004 | cli-mvp.md | Active 实施计划 | CLI 首版范围、协议契约和验收 |
| PLAN-001 | docs/cli-design.md | 用户技术方案 | 命令/API 映射、配置、输出、发送和交付 |
| FACT-001 | napcat-architecture.md | 源码观察 | Shell、桌面会话接入与未验证边界 |
| FACT-002 | napcat-release-license.md | 固定静态观察 | 版本、许可、真实附件与接口 |
| FACT-003 | native-loader-provenance.md | 历史及静态观察 | 原生引入/维护、构建来源和版本证据矩阵 |
| FACT-004 | framework-history-safety.md | 历史及静态观察 | 文档修改、官方答复、Bypass 与账号安全边界 |
| EVIDENCE-001 | evidence-register.md | 证据索引 | 公开来源和验证范围 |
| DATA-001 | napcat-v4.18.38.json | 静态清单 | 固定提交与附件及关键成员摘要 |
| DATA-002 | native-loader-provenance.json | 追溯数据 | 主线观察边界、历史 blob、PE 元数据与支持证据分类 |
| CHECK-001 | bootstrap-verification.md | 验证记录 | 本次初始化结果 |
| CHECK-002 | framework-audit-verification.md | 静态验证记录 | 下载校验、源码匹配与未运行边界 |
| CHECK-003 | native-loader-trace-verification.md | 追溯验证记录 | 历史、打包、相邻仓库及公开报告的调查范围 |
| CHECK-004 | framework-history-verification.md | 静态验证记录 | 文档历史、弃用范围及两种模式安全结论复核 |
| CHECK-005 | framework-runtime-trial.md | 实验记录 | 原版启动预检及新实例尝试，不保证会话隔离 |
| CHECK-006 | framework-message-focus-trial.md | 实验记录 | 用户指定会话的有界收发及匿名前台焦点观察 |
| CHECK-007 | cli-plan-verification.md | 文档验证记录 | CLI 方案交叉核对及检查范围 |
| CHECK-008 | cli-m0-verification.md | 离线验证记录 | 工具链、命令骨架、版本迁移及离线测试 |
| RFC-0000 | README.md | Active 流程 | 决策状态和接受条件 |
| RFC-0001 | 0001-session-loader.md | Draft RFC | 原生入口优先、自有加载器备选与不 fork 决策 |
| RFC-0002 | 0002-cli-api-client.md | Accepted RFC | Rust 技术栈，M0 已实现 |
| TODO-001 | TODO.md | Active 索引 | 实施清单入口 |
| TASK-001 | project.md | Active 清单 | 当前进度和后续验证差距 |
| TASK-002 | cli-mvp.md | Active 清单 | M0 完成，M1 至 M4 待实施 |

## 候选责任边界

| 层 | 预期职责 | 当前状态 |
|---|---|---|
| CLI | 命令、配置、JSON、错误、账号门禁及发送记录 | M0 解析/帮助/错误骨架已实现；配置/门禁/发送待实施 |
| 本地 API 适配 | 固定六个 OneBot action 与结果检查 | 有实验依据，正式客户端未实现 |
| 启动协调 | 由用户维护已有实例；原生入口研究独立处理 | 原版已实验，不进入 CLI MVP |
| 上游 NapCat | QQ 内部能力与协议适配 | 本轮不修改，不随 CLI 打包 |

当前有 Rust CLI M0 与固定工具链，后端用例尚未实现；不新增 QQ 操作或变更安装。
2026-10-17 的新增尝试授权优先：可新开原版 Framework 实例，由用户登录实验账号；不终止已有 QQ。
执行边界与结果由 CHECK-005 跟踪；此前静态调查的历史状态保留。
CHECK-005 已完成原版新实例登录及基础状态 API 验证，已有 QQ 进程仍在；没有发送或读取聊天。
这不代表完整同会话交互验收、产品实现或账号安全保证。
CHECK-006 已完成指定会话的 API/窗口发送与对方回复历史对照，以及短时焦点观察；未观察到调用期间抢焦点。
实时事件、更多消息类型、长期并行与恢复仍待验证，不把本次结果升级为通用保证。
不 fork；先澄清许可与原生可审计性，再按 SPEC-003 验证。静态入口存在不等于运行可用。
FACT-003 保留原版入口为有条件原型候选；当前证据不支持直接进入日用会话试跑。
