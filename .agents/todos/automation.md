# 自动化与上下文整理清单

文档编号：TASK-005。状态：本地实现与验收完成，远端运行待部署。日期：2026-10-22。
关联：RFC-0007、SPEC-002、SPEC-005、TASK-001、TASK-002。

- [x] 核对Semifold0.3.4本地命令与发布实现，确认私有包可GitHub Release但跳过发布命令，明确资产准备位置。
- [x] 同步自动化及上下文边界设计，保持编号与真实操作授权。
- [x] 将两份内部docs迁入.agents，修复入口与包范围并验收提交（CHECK-064；237文件源码包及公开文档链接检查通过）。
- [x] Windows可复用质量workflow及PR Semifold状态报告实现并静态验收（CHECK-065；actionlint 1.7.12通过）。
- [x] main Semifold CD、Windows资产打包/校验、workspace版本锁同步及仓库设置说明实现（CHECK-065）。
- [x] 完整本地质量、workflow语法、资产范围/版本/哈希及幂等拒绝验收后提交（CHECK-065；320项Rust、发布模块合成及实际ZIP/解包离线运行通过）。
- [ ] GitHub托管runner及真实Release端到端运行：需要远端部署，当前不执行，不冒称在线通过。

下一步：远端仓库就绪后按维护者授权部署、核对GitHub权限和首次版本PR/Release。SEMIFOLD_TOKEN为可选的App/PAT，未配置则遵守GITHUB_TOKEN的事件触发限制，可手动触发Quality；不擅自创建远端或配置令牌。本任务不提升业务混合清单完成比例，稳定性仍暂缓。

README-001已改为公开用户入口：说明安装启动、消息操作、Agent指引、多实例及使用边界，移除内部编号与实验流水；按Clap命令定义核对示例、检查本地链接和分发清单，未执行真实QQ操作。此文档整理不改变业务验收状态，稳定性仍暂缓。
