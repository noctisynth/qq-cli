# 内部上下文迁移与分发边界

文档编号：CHECK-064。状态：通过。日期：2026-10-22。
关联：RFC-0007、SPEC-002、SPEC-005、TASK-005。

用户授权将docs中的Agent上下文归入.agents。DESIGN-002迁为.agents/architecture.md，PLAN-001迁为.agents/specs/cli-design.md；编号与历史证据保留，不把内部阶段记录包装成用户手册。README改为Skill/Schema使用入口；CONTRIBUTING与DESIGN索引改为实际内部路径。

Cargo include移除docs，源码白名单同步排除docs。cargo package --list --allow-dirty --offline --locked得到237个文件，白名单通过；内部上下文、运行研究材料未入包。公开README/CONTRIBUTING的本地Markdown链接存在检查及旧docs路径检索通过，git diff --check通过。没有运行QQ或版本消费。

因源码分发内容与公开文档变化创建patch:chore；纯内部设计/清单无额外changeset。CI/CD规范与专题任务同步，但workflow和在线执行尚未实现，不据迁移通过提升为发布完成。下一步Windows质量与Semifold自动化。
