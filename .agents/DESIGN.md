# 工程上下文导航

文档编号：DESIGN-001。状态：Active。更新：2026-10-24。
协作指引：[AGENTS.md](../AGENTS.md)。当前工作：[TODO-001](TODO.md)。代码导航：[DESIGN-002](architecture.md)。

## 按任务阅读

- 了解产品目标和命令职责：RFC-0002/0003/0004 → PLAN-001。
- 修改运行管理：SPEC-006 → DESIGN-002 → TASK-002；需要验证依据时再查 CHECK。
- 开发 Apple Silicon macOS：RFC-0008 → FACT-005 → SPEC-008 → TASK-006；原型证据 CHECK-072。
- 修改公共类型/Schema：RFC-0006 → SPEC-007 → TASK-004。
- 修改发布：RFC-0007 → SPEC-002/005 → TASK-005。
- 查历史实验：[EVIDENCE-001](evidence/evidence-register.md)。历史失败按当次版本解释，不覆盖后续限定成功。

当前交付范围和缺口由任务清单维护，不在设计索引复制实验摘要或维护第二份实现计数。

## 需求方案

| 编号 | 文档 |
|---|---|
| RFC-0001 | [无 LiteLoader 的桌面会话接入](rfcs/0001-session-loader.md) |
| RFC-0002 | [面向 coding agent 的 QQ CLI 需求](rfcs/0002-cli-api-client.md) |
| RFC-0003 | [自动管理 NapCat 安装与实例](rfcs/0003-managed-runtime.md) |
| RFC-0004 | [CLI 语义、职责与调用成本](rfcs/0004-cli-ux-consolidation.md) |
| RFC-0005 | [Rust 模块职责与可维护性](rfcs/0005-maintainable-architecture.md) |
| RFC-0006 | [CLI 数据契约单一来源](rfcs/0006-single-source-contracts.md) |
| RFC-0007 | [自动检查、持续交付与分发边界](rfcs/0007-automation-delivery.md) |
| RFC-0008 | [Apple Silicon macOS 支持需求](rfcs/0008-platform-runtime.md) |
| RFC-0000 | [需求方案工作流](rfcs/README.md) |

## 技术方案

| 编号 | 文档 |
|---|---|
| SPEC-005 | [自动检查与发布技术方案](specs/automation.md) |
| PLAN-001 | [CLI 命令与数据模型](specs/cli-design.md) |
| SPEC-004 | [领域、媒体与事件技术契约](specs/cli-product.md) |
| SPEC-007 | [类型、Schema 与输入规则技术方案](specs/data-contracts.md) |
| SPEC-003 | [Framework 接入与验证技术方案](specs/framework-integration-plan.md) |
| SPEC-008 | [Apple Silicon Framework 运行适配方案](specs/macos-runtime.md) |
| SPEC-001 | [CLI 技术基线与系统边界](specs/product-baseline.md) |
| SPEC-002 | [仓库、版本与上下文分发配置](specs/repository-bootstrap.md) |
| SPEC-006 | [安装、运行身份与生命周期技术方案](specs/runtime.md) |

## 调研事实

| 编号 | 文档 |
|---|---|
| FACT-004 | [Framework 文档历史、弃用范围与账号风险](facts/framework-history-safety.md) |
| FACT-005 | [Apple Silicon macOS 接入事实](facts/macos-portability.md) |
| FACT-001 | [NapCat 架构研究基线](facts/napcat-architecture.md) |
| FACT-002 | [NapCat 固定版本、许可与产物核对](facts/napcat-release-license.md) |
| FACT-003 | [Framework 原生加载器来源与维护追溯](facts/native-loader-provenance.md) |

## 实施清单

| 编号 | 文档 |
|---|---|
| TASK-005 | [自动化与发布清单](todos/automation.md) |
| TASK-002 | [CLI 产品实现与验收清单](todos/cli-product.md) |
| TASK-003 | [代码质量整改清单](todos/code-quality.md) |
| TASK-006 | [Apple Silicon macOS 实施与验收清单](todos/macos-portability.md) |
| TASK-001 | [项目当前工作清单](todos/project.md) |
| TASK-004 | [单源契约整改清单](todos/single-source-contracts.md) |

## 上下文维护规则

AGENTS.md 是无编号的核心协作指引，不属于下表中的工程文档编号体系。

| 类型 | 唯一职责 | 不应承载 |
|---|---|---|
| RFC | 需求方案：问题、目标、非目标、用户行为、取舍及验收要求 | 实现流水、详细接口和测试日志 |
| Spec / PLAN | 技术方案：接口、数据模型、状态机、平台机制、错误及兼容策略 | 临时授权、阶段日志和重复需求全文 |
| FACT | 有来源且注明适用基线的事实结论；区分观察与推断 | 实施指令和任务勾选 |
| EVIDENCE / CHECK | 当次验证的方法、版本、结果和限制，供追溯与复核 | 无范围的当前支持承诺 |
| TASK | 目标与实现/验收之间的差距；完成条件和证据引用 | 详细实验过程 |
| DESIGN / TODO | 文档导航 / 当前工作总览 | 重复专题正文和历史流水 |

每类信息只维护一个主要位置，其余引用文档编号或相对链接。编号保留，不依赖文件名表达权威。
需求变化更新对应 RFC，技术机制变化更新对应 Spec，事实变化更新 FACT，实验结果进入 CHECK，
任务状态变化更新对应 TASK。核对相关文档的一致性，不要求每次 CHECK 修改所有上下文。
总览只有在索引或当前优先级确实改变时更新；不复制实验摘要，不维持多份任务计数。
历史事实/证据保留原日期和版本，后续结论注明替代关系；当前方案正文不堆叠已失效的“本轮”指令。
Accepted 表示需求方案已采纳，不等于实现或验收完成；部分完成拆开记录，不把离线通过写成真机通过。
