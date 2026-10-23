# 仓库、版本与上下文分发配置

文档编号：SPEC-002。状态：Active。更新：2026-10-24。
需求：RFC-0007。发布机制：SPEC-005。历史初始化证据：CHECK-001、CHECK-008。

Git 主分支 main；Semifold 管理单发布单元 qq-cli、release 分支和 .changes。
Cargo.toml 是唯一版本来源，使用 rust resolver；提交 Cargo.lock，工具链与 rust-version 保持一致。
不维护 package.json 版本。当前发布状态见 TASK-005，本文不保存随发布变化的版本状态副本。

影响发布内容、产品行为或使用方式的变更创建 changeset；用户指南用 patch:chore。
纯 Agent 上下文不创建 changeset；不回填已清除的历史研究条目，不手工消费或本地执行 version/publish。
原创代码采用 MIT OR Apache-2.0，源码及二进制分发边界由 SPEC-005 统一定义。

内部上下文位于 .agents，文档职责由 DESIGN-001 定义，核心协作规则见 [AGENTS.md](../../AGENTS.md)。临时工作、研究产物、日志、QQ/NapCat 下载、
凭据与私人会话留忽略目录；公开上下文只能保存原创、脱敏、可追溯的结论。
文档变更检查编号、引用、事实范围、敏感信息及 diff；代码变更增加对应门禁。
