# 仓库自动化与发布契约

文档编号：SPEC-005。状态：Active。日期：2026-10-22。
关联：RFC-0007、SPEC-002、TASK-005。

- PR与手动触发运行可复用Windows质量检查；main推送先通过相同质量门禁再运行Semifold。检查包括结构/单源契约、fmt、Clippy、完整串行合成测试、release构建和源码包范围，不读取真实QQ会话。
- PR状态由semifold status报告；同仓PR允许写评论，fork PR只做无写权限质量检查，不使用pull_request_target执行贡献者代码。
- Semifold独占版本、发布分支/PR、tag、changelog与GitHub Release编排。ci输出保留semifold-version/semifold-publish结构化事实，不从tag或日志重新推断发布结果；发布阶段不并发取消。
- 交付目标Windows x86_64-pc-windows-msvc，版本只读Cargo.toml，产物ZIP仅原创CLI、README、双许可证及Skill；独立SHA256SUMS校验ZIP。发布文件不含NapCat/QQ、研究材料、.agents或凭据。
- Cargo及Semifold启用crates.io发布，resolver显式执行cargo publish --locked；CARGO_REGISTRY_TOKEN仅注入受信发布步骤。原创代码采用用户选择的MIT OR Apache-2.0，双许可证纳入源码包及ZIP。Semifold同时管理GitHub Release及显式资产。post-version更新workspace锁记录，保持第三方锁定版本。发布所需GitHub权限仅授予受信main发布job。
- 内部上下文统一.agents，源码包不再包括docs；公开文档只说明使用、开发命令和仓库自动化。仓库已公开推送并触发远端CI；真实crates.io与GitHub Release发布尚待版本流程，不提前标记通过。

README用户入口约定（2026-10-22）：核心定位为AI Agent通过Shell调度QQ；优先提供可交给Agent执行的配置请求和自然语言任务示例，手动安装/命令作为补充。公开用户文档保留实际操作要求、明确语义和已确认问题，不混入验证推理、验收流水或泛化免责声明；证据边界保留内部上下文。README已按此整理，本地链接、runtime指引入口、源码包和措辞检查通过；业务与稳定性验收状态不变。

公开入口补充（2026-10-22）：展示名称统一QQ CLI，命令/包名保留qq-cli。README面向GitHub访客，通过复制仓库链接委托本地Agent获取、安装和配置，不要求用户先克隆或使用Git；末尾鸣谢NapCat。当前无远端地址，使用明确替换的仓库链接占位，不虚构URL。文档链接、命令名保全及diff检查通过，patch:chore登记；实现与验收状态不变。

安装入口扩展（2026-10-22）：新增独立scripts/install.ps1，支持Windows PowerShell 5.1及PowerShell 7，在Windows x64从官方仓库latest稳定Release解析固定资产，校验SHA256后只安装qq-cli.exe至用户目录，更新用户/当前进程PATH；重复运行升级，下载或校验失败保留旧程序，支持自定义目录与NoPath。无需Git/gh/Rust/管理员/token，不启动QQ、不安装NapCat。脚本可dot-source供合成测试，离线验证加入质量门禁；README给Agent直接运行入口。当前阶段待实现与验收。

首次发布版本与上游排查（2026-10-22）：用户要求首次稳定发布保持0.1.0，不能按现有默认计划升至0.2.0。已取消运行39663937957的后续流程；取消前完整Rust测试已通过，包括原生启动恢复用例，不能据此宣称偶发根因已修复。Semifold0.3.4及上游main固定提交41fb100449a14018e2b6de1290b97a60c4e5d3c2的planner::bump_with_policy仅在进入命名预发布通道时应用channel-bump=preserve，没有保持稳定版的对应入口；不得用alpha等预发布替代0.1.0稳定版。首次发布实现路径待用户选择上游支持或一次性流程，期间不合并、不发布、不推送触发默认升版的提交。上游ci::force_push_release未注册push_update_reference回调，也未验证远端分支存在，存在服务端拒绝未报告的诊断缺口；本仓旧运行基线缺少main后来新增的workflow令牌环境配置，可能遇到GITHUB_TOKEN修改workflow限制，但没有服务端拒绝原文，不能断言该推测已确认。
