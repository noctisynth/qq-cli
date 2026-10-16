# qq-cli 工程设计索引

文档编号：DESIGN-001。状态：Active。更新：2026-10-16。

## 产品方向与当前状态

面向个人 QQ 的本地 Agent 命令行接入，保留官方桌面窗口，与用户共用同一会话，不抢占输入焦点。
不依赖 LiteLoaderQQNT，不创建独立官方机器人身份。当前完成工程初始化与固定版本静态审计，未实现产品。
NapCat v4.18.38 的 Framework 包已有原生加载入口，优先评估原版，不必立即开发加载器或 fork。
窗口保留、共享会话仍未实测；已追溯原生文件分发历史，但原始编译来源、适用许可和版本支持仍有缺口。
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
| SPEC-003 | framework-integration-plan.md | Active 验证计划 | 原版入口、生命周期及分阶段验收 |
| FACT-001 | napcat-architecture.md | 源码观察 | Shell、桌面会话接入与未验证边界 |
| FACT-002 | napcat-release-license.md | 固定静态观察 | 版本、许可、真实附件与接口 |
| FACT-003 | native-loader-provenance.md | 历史及静态观察 | 原生引入/维护、构建来源和版本证据矩阵 |
| EVIDENCE-001 | evidence-register.md | 证据索引 | 公开来源和验证范围 |
| DATA-001 | napcat-v4.18.38.json | 静态清单 | 固定提交与附件及关键成员摘要 |
| DATA-002 | native-loader-provenance.json | 追溯数据 | 主线观察边界、历史 blob、PE 元数据与支持证据分类 |
| CHECK-001 | bootstrap-verification.md | 验证记录 | 本次初始化结果 |
| CHECK-002 | framework-audit-verification.md | 静态验证记录 | 下载校验、源码匹配与未运行边界 |
| CHECK-003 | native-loader-trace-verification.md | 追溯验证记录 | 历史、打包、相邻仓库及公开报告的调查范围 |
| RFC-0000 | README.md | Active 流程 | 决策状态和接受条件 |
| RFC-0001 | 0001-session-loader.md | Draft RFC | 原生入口优先、自有加载器备选与不 fork 决策 |
| TODO-001 | TODO.md | Active 索引 | 实施清单入口 |
| TASK-001 | project.md | Active 清单 | 当前进度和后续验证差距 |

## 候选责任边界

| 层 | 预期职责 | 当前状态 |
|---|---|---|
| CLI | 命令、参数、结构化结果与错误 | 未设计、未实现 |
| 本地 API 适配 | 与 NapCat 接口通信、结果确认 | 版本已固定，运行契约待验证 |
| 启动协调 | 优先原版原生入口，必要时评估自有加载器 | 候选方案，未运行 |
| 上游 NapCat | 内部消息能力与协议适配 | 已下载静态审计，未执行或集成 |

上述职责划分用于研究，不冻结语言或组件数量。本轮不启动 QQ、不加载 DLL、不变更 QQ 安装。
不 fork；先澄清许可与原生可审计性，再按 SPEC-003 验证。静态入口存在不等于运行可用。
FACT-003 保留原版入口为有条件原型候选；当前证据不支持直接进入日用会话试跑。
