# 仓库与版本初始化

文档编号：SPEC-002。状态：Active。更新：2026-10-17。

建立独立 Git 仓库，初始分支 main；初始化阶段仅本地提交，不配置远端。用户2026-10-22授权创建GitHub自动检查及Semifold CD，最新契约见SPEC-005/RFC-0007，执行差距见TASK-005；不因此授权推送或实际发布。
使用 Semifold 管理 .changes，基础分支 main，发布分支 release。
单发布单元 qq-cli，当前版本 0.1.0（尚未发布），唯一来源为根 Cargo.toml，Semifold 使用 rust 解析器。
初始化曾使用 package.json/nodejs 元数据；TASK-002 的 M0 已完成迁移并移除旧文件，见 CHECK-008。
Cargo.lock 提交，rust-toolchain.toml 和 rust-version 当前均为 1.98.1，不因此约束 QQ 内部运行时。
保留包注册表发布禁用；GitHub Release改为由Semifold CD管理自产Windows CLI资产，配置和验收按TASK-005实施。既往禁用GitHub Release是初始化状态，不覆盖最新用户授权。
初始化时曾建立 patch / chore changeset；2026-10-17 按用户要求移除纯上下文及研究记录条目。
影响发布内容、产品行为或使用方式的代码/配置变更创建 changeset；用户文档使用 patch:chore。
纯 Agent 上下文不创建 changeset；历史已清除条目不回填。是否文档不单由扩展名判断，以读者和用途为准。
不执行版本提升或消费。

上下文遵循 GOV-001 的类型边界。临时工作、日志、凭据、私人会话、QQ/NapCat 下载和二进制
不进入提交；上游依赖的许可及来源核对在实际集成之前完成。
初始化验收要求配置可解析、无待同步包差异、文档引用完整和 Git diff 无空白错误。
存在发布变更时检查 changeset 可识别；纯研究阶段无待发布 changeset 为正常状态。
