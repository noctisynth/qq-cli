# 自动检查与发布技术方案

文档编号：SPEC-005。状态：Active。更新：2026-10-30。
需求：RFC-0007。配置基线：SPEC-002。验收：TASK-005；历史过程：CHECK-064 至 CHECK-066。

- PR与手动触发运行可复用Windows及原生Apple Silicon质量检查；main推送先通过相同质量门禁再运行Semifold。检查包括结构/单源契约、fmt、Clippy、完整串行合成测试、release构建和源码包范围，不读取真实QQ会话。
- PR状态由semifold status报告；同仓PR允许写评论，fork PR只做无写权限质量检查，不使用pull_request_target执行贡献者代码。
- Semifold独占版本、发布分支/PR、tag、changelog与GitHub Release编排。ci输出保留semifold-version/semifold-publish结构化事实，不从tag或日志重新推断发布结果；发布阶段不并发取消。
- 交付目标Windows x86_64-pc-windows-msvc及macOS aarch64-apple-darwin，版本只读Cargo.toml，产物ZIP仅原创CLI、README、双许可证及Skill；独立SHA256SUMS校验ZIP。发布文件不含NapCat/QQ、研究材料、.agents或凭据。
- Cargo及Semifold启用crates.io发布，resolver显式执行cargo publish --locked；CARGO_REGISTRY_TOKEN仅注入受信发布步骤。原创代码采用用户选择的MIT OR Apache-2.0，双许可证纳入源码包及ZIP。Semifold同时管理GitHub Release及显式资产。post-version更新workspace锁记录，保持第三方锁定版本。发布所需GitHub权限仅授予受信main发布job。
- 内部上下文统一.agents，源码包不再包括docs；公开文档只说明使用、开发命令和仓库自动化。

三个workflow分别承担quality、semifold-status、semifold-ci；Windows质量复用scripts/check.ps1，macOS质量复用scripts/check-macos.sh。
固定 Rust 工具链取 rust-toolchain.toml，Cargo 先 fetch --locked 再离线验收。
Semifold setup 在各 workflow 使用一致的默认 latest 策略；通用 Actions 使用明确主版本。
main 发布串行编排，Semifold 前准备自产资产；临时 setup 脚本不得留在工作区被版本提交收集。
post-version 仅用 cargo update --workspace --offline 同步自身版本，不改变第三方锁定依赖。

内部上下文和实验材料不发布。公开 README 面向 Agent 使用任务，CONTRIBUTING 说明开发命令，Skill 按任务提供指引。
当前发布验收状态只在 TASK-005 维护；不在技术方案中追加每次 CI/PR 的执行流水。

## 双平台构建与发布资产

quality复用workflow另设macos-15 job，显式核对uname arm64；固定仓库Rust工具链与Node 22，缓存按平台分离。先fetch locked依赖，再执行check-macos.sh和package-macos.sh；两平台的ZIP和各自校验清单按提交SHA分别上传并保留7天，不将配置落地等同托管runner验收。所有质量job完成后，Semifold发布job才可继续。

package-macos.sh只组合原生目标构建、Cargo元数据和实际CLI版本检查；macos_release.py只负责原创文件白名单、thin arm64 Mach-O形状、普通文件/链接拒绝、固定时间/Unix模式ZIP和SHA-256完整校验。输出独占创建，失败保留，不覆盖或隐式发布。合成检查覆盖错误架构/载荷、额外文件、权限/校验变化和输出保全。Python只属于开发/分发工具，不成为CLI运行依赖。

Semifold发布job通过download-artifact取得本次质量运行的Windows/macOS包，在读取Cargo唯一版本后调用scripts/release_assets.py。汇总器要求各平台目录恰有对应版本ZIP及原SHA256SUMS，核对原摘要后原样复制两个归档，以独占创建生成公共SHA256SUMS；错误版本、篡改、额外文件、链接及既有输出拒绝，不执行包内程序、不上传文件。Semifold资产列表包含两个ZIP和公共清单，两种安装器均自动选择本平台包并校验，无需用户手动核实哈希。

Python仅用于开发及发布job，最终用户不需要它、Git、Rust或仓库副本。公开文档提供正式安装方式，不承载验收流水或临时开发状态；内部TASK继续区分本地验证、托管执行及线上发布结果。实际远端发布与线上安装结果由TASK-006跟踪，不以配置或合成通过冒充已发布。

## macOS安装与CLI版本保全

install-macos.sh将Release发现、归档验证及版本发布分为独立职责；依赖系统Bash/curl/JXA/ditto，不要求最终用户Python/Git/Rust。不带归档参数时仅接受同仓稳定Release的唯一macos-aarch64 ZIP及SHA256SUMS；资产缺失、重复或来源不符时在安装目录效果前失败。离线模式显式提供ZIP和校验清单。先验证SHA-256及完整原创文件名单，再解包、拒绝链接、检查thin arm64头及执行位。安装器同时接受原九文件Skill布局和扩展后十四文件Skill布局，分别要求完整匹配；缺页、混合子集或额外文件均拒绝，保留main安装器对既有稳定Release的安装/升级能力。使用系统ditto的quarantine传播，不清除属性；系统可重建时间/来源字段。

独立私有安装目录以固定marker和协作式安装锁区分所有权，版本槽由归档摘要派生。新槽发布后原子切换受管命令链接；已有非受管命令、变化槽或未知锁拒绝覆盖。保留旧槽和打印前版位置，不把入口更新等同Keychain访问迁移。卸载只移除本安装拥有的命令链接，不删除版本、QQ数据或凭据，不改shell配置/系统保护。默认安装根与运行状态根分离；锁未知时不自动重放。

合成安装/升级/卸载及模拟Release发现用于验证安装协议，不证明线上发行或所有跨签名Keychain组合。固定真机上的本地包安装升级、命令链接访问原条目、显式系统授权恢复和卸载保全已另行验收；证据与扩展范围见TASK-006。
