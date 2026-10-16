# 仓库与版本初始化

文档编号：SPEC-002。状态：Active。更新：2026-10-16。

建立独立 Git 仓库，初始分支 main；本次仅本地提交，不配置远端和 GitHub 自动化。
使用 Semifold 管理 .changes，基础分支 main，发布分支 release。
单发布单元 qq-cli，初始版本 0.1.0（尚未发布），由根 package.json 承载。
使用 nodejs 解析器读取版本元数据；不因此确定 CLI 或加载器的运行时。
禁用包注册表发布和 GitHub Release，后续发布设计独立确定。
初始化 changeset 使用 patch / chore；不执行版本提升或消费。

上下文遵循 GOV-001 的类型边界。临时工作、日志、凭据、私人会话、QQ/NapCat 下载和二进制
不进入提交；上游依赖的许可及来源核对在实际集成之前完成。
初始化验收要求配置可解析、无待同步包差异、changeset 可识别、文档引用完整和 Git diff 无空白错误。
