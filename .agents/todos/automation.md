# 自动化与上下文整理清单

文档编号：TASK-005。状态：安装与发布配置已验收，远端已部署、首次发布待完成。日期：2026-10-22。
关联：RFC-0007、SPEC-002、SPEC-005、TASK-001、TASK-002。

- [x] 核对Semifold0.3.4本地命令与发布实现，确认私有包可GitHub Release但跳过发布命令，明确资产准备位置。
- [x] 同步自动化及上下文边界设计，保持编号与真实操作授权。
- [x] 将两份内部docs迁入.agents，修复入口与包范围并验收提交（CHECK-064；237文件源码包及公开文档链接检查通过）。
- [x] Windows可复用质量workflow及PR Semifold状态报告实现并静态验收（CHECK-065；actionlint 1.7.12通过）。
- [x] main Semifold CD、Windows资产打包/校验、workspace版本锁同步及仓库设置说明实现（CHECK-065）。
- [x] 完整本地质量、workflow语法、资产范围/版本/哈希及幂等拒绝验收后提交（CHECK-065；320项Rust、发布模块合成及实际ZIP/解包离线运行通过）。
- [ ] GitHub托管runner及真实Release端到端运行：已创建公开仓库并触发CI；crates.io与GitHub Release首次发布、latest在线安装待完成。

下一步：远端仓库就绪后按维护者授权部署、核对GitHub权限和首次版本PR/Release。SEMIFOLD_TOKEN为可选的App/PAT，未配置则遵守GITHUB_TOKEN的事件触发限制，可手动触发Quality；不擅自创建远端或配置令牌。本任务不提升业务混合清单完成比例，稳定性仍暂缓。

README-001已改为公开用户入口：说明安装启动、消息操作、Agent指引、多实例及使用边界，移除内部编号与实验流水；按Clap命令定义核对示例、检查本地链接和分发清单，未执行真实QQ操作。此文档整理不改变业务验收状态，稳定性仍暂缓。

README用户入口约定（2026-10-22）：核心定位为AI Agent通过Shell调度QQ；优先提供可交给Agent执行的配置请求和自然语言任务示例，手动安装/命令作为补充。公开用户文档保留实际操作要求、明确语义和已确认问题，不混入验证推理、验收流水或泛化免责声明；证据边界保留内部上下文。README已按此整理，本地链接、runtime指引入口、源码包和措辞检查通过；业务与稳定性验收状态不变。

公开入口补充（2026-10-22）：展示名称统一QQ CLI，命令/包名保留qq-cli。README面向GitHub访客，通过复制仓库链接委托本地Agent获取、安装和配置，不要求用户先克隆或使用Git；末尾鸣谢NapCat。当前无远端地址，使用明确替换的仓库链接占位，不虚构URL。文档链接、命令名保全及diff检查通过，patch:chore登记；实现与验收状态不变。

远端部署进度（2026-10-22）：用户明确授权gh创建noctisynth/qq-cli公开仓库并推送；仓库已创建，README配置请求及Releases已替换为实际公开URL。本次提交准备推送main；GitHub Actions及真实Release仍以远端运行结果为准，不提前标记TASK-005端到端通过。

安装入口扩展（2026-10-22）：新增独立scripts/install.ps1，支持Windows PowerShell 5.1及PowerShell 7，在Windows x64从官方仓库latest稳定Release解析固定资产，校验SHA256后只安装qq-cli.exe至用户目录，更新用户/当前进程PATH；重复运行升级，下载或校验失败保留旧程序，支持自定义目录与NoPath。无需Git/gh/Rust/管理员/token，不启动QQ、不安装NapCat。脚本可dot-source供合成测试，离线验证加入质量门禁；README给Agent直接运行入口。当前阶段待实现与验收。

安装与注册表发布阶段完成（2026-10-22）：独立install.ps1从latest稳定Release校验SHA256并安装exe及双许可证，默认用户Programs/qq-cli并去重更新用户/当前PATH；NoPath和自定义目录可用。PS7/Windows PowerShell5.1合成安装、升级、重复、校验/架构/锁定失败保全、ZIP路径隔离、临时清理、PATH及无发布包失败通过；完整编排用假下载验证，不修改用户PATH。真实自产12文件ZIP安装后version/skill冒烟通过，Cargo源码包244文件独立编译通过；actionlint与Semifold配置检查通过。用户已设置CARGO_REGISTRY_TOKEN并选择MIT OR Apache-2.0，发布工作流及Cargo元数据已接入，不读取secret值。真实Release尚不存在，在线latest安装与crates.io实际发布待后续版本流程；不自动合并版本PR。


- [x] 一键安装脚本、PS5.1/PS7离线测试、README入口及真实自产ZIP安装冒烟。
- [x] MIT OR Apache-2.0双许可、源码/ZIP许可证、Semifold crates.io命令及令牌环境接入。

首次发布推进（2026-10-22）：用户授权核对README/Skill、修复CI、通过后合并版本PR并首次发布crates.io/GitHub Release及在线安装验收。已补齐Skill的安装器、PATH恢复、无配置默认值、导出目录、固定NapCat版本、首次登录与恢复流程；删除内部实验叙述和过时媒体验收状态。Skill三项离线检查通过（导出需正常用户ACL权限）；初次线上质量步骤通过，打包/首次发布仍进行中。未授权新增真实QQ操作。

配置流程补核：Skill导出实现要求父目录已存在、目标目录不存在；README和runtime指引已补充，避免新Agent直接导出到缺失宿主目录而失败。

CI排查阶段（2026-10-22）：旧流水线质量/打包通过，但Semifold创建PR收到422 head invalid且远端无release分支；版本推送原因继续核对。最新流水线98个库测试中原生启动恢复用例出现None unwrap，原始dispatch错误被测试吞掉；修正测试直接报告launch/journal错误并添加仅测试编译的阶段诊断。本地40次定向及98项库测试、Clippy、结构通过，线上根因尚待复核，不标记已修复。单独授权的好友状态说明已发送一次accepted；对象/正文仅忽略材料，查询单次超时可通过临时60秒配置完成；最近会话不支持类型另待查明。

首次发布版本与上游排查（2026-10-22）：用户要求首次稳定发布保持0.1.0，不能按现有默认计划升至0.2.0。已取消运行39663937957的后续流程；取消前完整Rust测试已通过，包括原生启动恢复用例，不能据此宣称偶发根因已修复。Semifold0.3.4及上游main固定提交41fb100449a14018e2b6de1290b97a60c4e5d3c2的planner::bump_with_policy仅在进入命名预发布通道时应用channel-bump=preserve，没有保持稳定版的对应入口；不得用alpha等预发布替代0.1.0稳定版。首次发布实现路径待用户选择上游支持或一次性流程，期间不合并、不发布、不推送触发默认升版的提交。上游ci::force_push_release未注册push_update_reference回调，也未验证远端分支存在，存在服务端拒绝未报告的诊断缺口；本仓旧运行基线缺少main后来新增的workflow令牌环境配置，可能遇到GITHUB_TOKEN修改workflow限制，但没有服务端拒绝原文，不能断言该推测已确认。

配置实测补充（2026-10-22）：按用户指令在.changes/config.toml的packages.qq-cli中显式加入channel-bump=preserve，未设置预发布通道。Semifold0.3.4 status成功读取配置，但仍规划0.1.0→0.2.0；仅完成配置写入，首次稳定版保持0.1.0尚未实现。已登记配置changeset，禁止据此认定可发布；本次仅本地提交、不推送触发CD。

Alpha转正验证（2026-10-22）：用户授权隔离验证后使用未发布的0.1.0-alpha.0作为首次稳定发布过渡。隔离仓库复制全部115个changeset，Semifold0.3.4 status及实际version应用均得到0.1.0，post-version锁文件同步成功。正式Cargo.toml及Cargo.lock已同步为0.1.0-alpha.0，第三方依赖不变；含新增条目的116个changeset规划目标仍为0.1.0。允许继续推送、CI、稳定版本PR合并及首次发布，替代此前等待方案选择的暂停；不发布任何alpha版本。首次发布与在线安装仍待远端验证。
