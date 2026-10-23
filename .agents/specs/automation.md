# 自动检查与发布技术方案

文档编号：SPEC-005。状态：Active。更新：2026-10-24。
需求：RFC-0007。配置基线：SPEC-002。验收：TASK-005；历史过程：CHECK-064 至 CHECK-066。

- PR与手动触发运行可复用Windows质量检查；main推送先通过相同质量门禁再运行Semifold。检查包括结构/单源契约、fmt、Clippy、完整串行合成测试、release构建和源码包范围，不读取真实QQ会话。
- PR状态由semifold status报告；同仓PR允许写评论，fork PR只做无写权限质量检查，不使用pull_request_target执行贡献者代码。
- Semifold独占版本、发布分支/PR、tag、changelog与GitHub Release编排。ci输出保留semifold-version/semifold-publish结构化事实，不从tag或日志重新推断发布结果；发布阶段不并发取消。
- 交付目标Windows x86_64-pc-windows-msvc，版本只读Cargo.toml，产物ZIP仅原创CLI、README、双许可证及Skill；独立SHA256SUMS校验ZIP。发布文件不含NapCat/QQ、研究材料、.agents或凭据。
- Cargo及Semifold启用crates.io发布，resolver显式执行cargo publish --locked；CARGO_REGISTRY_TOKEN仅注入受信发布步骤。原创代码采用用户选择的MIT OR Apache-2.0，双许可证纳入源码包及ZIP。Semifold同时管理GitHub Release及显式资产。post-version更新workspace锁记录，保持第三方锁定版本。发布所需GitHub权限仅授予受信main发布job。
- 内部上下文统一.agents，源码包不再包括docs；公开文档只说明使用、开发命令和仓库自动化。

三个 workflow 分别承担 quality、semifold-status、semifold-ci；质量复用 scripts/check.ps1。
固定 Rust 工具链取 rust-toolchain.toml，Cargo 先 fetch --locked 再离线验收。
Semifold setup 在各 workflow 使用一致的 0.3.x 策略；通用 Actions 使用明确主版本。
main 发布串行编排，Semifold 前准备自产资产；临时 setup 脚本不得留在工作区被版本提交收集。
post-version 仅用 cargo update --workspace --offline 同步自身版本，不改变第三方锁定依赖。

内部上下文和实验材料不发布。公开 README 面向 Agent 使用任务，CONTRIBUTING 说明开发命令，Skill 按任务提供指引。
当前发布验收状态只在 TASK-005 维护；不在技术方案中追加每次 CI/PR 的执行流水。
