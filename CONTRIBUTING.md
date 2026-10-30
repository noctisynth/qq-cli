# 贡献指南

## 上下文

从 [AGENTS.md](AGENTS.md) 读取协作规则，再按 [.agents/DESIGN.md](.agents/DESIGN.md) 定位相关需求和技术方案，当前任务见 [.agents/TODO.md](.agents/TODO.md)。工程上下文不随源码包或 CLI 压缩包发布。

修改Rust代码先读内部[架构与维护入口](.agents/architecture.md)。源码按app/cli/core/storage/requests/messaging/napcat/native/transport组织；跨层编排、原生边界和测试归属遵循该文档。RFC-0005/TASK-003记录已完成的质量整改。

## 当前工具

需要 Git、Semifold 和 rustup；Windows 使用 MSVC 链接工具，Apple Silicon macOS 使用 Xcode Command Line Tools。Rust 工具链由 rust-toolchain.toml 固定。
双平台开发门禁需要 Node.js 22 或更新版本运行原创 Framework 适配器合成测试，无需 npm 包；最终用户运行 CLI 不需要外部 Node.js。
Cargo.toml 是唯一包版本来源，Semifold 使用 rust 解析器，旧 package.json 已移除。
Cargo.lock 锁定依赖；HTTP 仅接入已登记的字面回环地址，禁用代理、重定向和自动重试。

## 本地构建

以下命令在仓库根目录执行；最终用户使用 README 中的一键安装入口。

| 平台 | 构建命令 | 可执行文件 |
| --- | --- | --- |
| Windows x64 | `cargo build --release --locked` | `target/release/qq-cli.exe` |
| Apple Silicon macOS | `cargo build --release --locked --target aarch64-apple-darwin` | `target/aarch64-apple-darwin/release/qq-cli` |

## 本地检查

首次准备可联网执行 cargo fetch --locked；随后运行以下离线检查：

```powershell
./scripts/check.ps1
```

脚本依次执行原创适配器合成测试、架构/契约边界、合成发布包、安装器与版本锁同步检查、fmt、clippy（警告视为错误）、全部串行单元/集成测试、release 构建和源码包白名单检查；构建/测试/包清单使用 --offline --locked。
测试使用合成数据、临时私有目录、内存后端和有截止的回环假服务，不需要运行 QQ/NapCat，也不读取真实配置或聊天。
Windows 私有 ACL 和回环监听检查需要普通用户的对应系统权限；受限沙箱可能拒绝，不能通过削弱权限或跳过测试代替验证。

Apple Silicon macOS 开发检查（先执行 `cargo fetch --locked`）：

```sh
./scripts/check-macos.sh
```

macOS 构建使用 Apple Command Line Tools 的 Clang 与系统 SDK 编译 socket、audit token 信号、进程发现等窄原生适配。CLI 与业务逻辑仍使用 Rust，原生边界见架构文档。
macOS 开发门禁需要 Python 3.9 或更新版本运行原创分发合成检查，还需要 Node.js 22 或更新版本，以执行原创 Framework 薄适配的合成测试；无需安装 npm 包，也不读取或启动 QQ。这是开发检查依赖，CLI 不依赖外部 Node.js 运行时。签名负例测试使用系统 `codesign` 对临时目录中的原创测试程序副本做 ad-hoc 签名，再确认其不能冒充官方 QQ；不修改 QQ 应用。

该入口运行分发包合成检查、原生 arm64 的 fmt、Clippy、全部可编译测试及 release 构建，自动解析临时目录的系统别名，保持私有状态拒绝符号链接的规则。测试需要回环监听及创建原创子进程的权限，不访问 QQ。TLS 测试为每次运行生成短期证书，仅向合成客户端添加信任，不修改系统证书库。
Windows 原生、安装器和发布包检查仍执行 `check.ps1`。真实 QQ 操作须另按当前授权范围进行，离线检查不会启动 QQ。

常规检查：git diff --check、semifold config sync --check、semifold status、git status --short。
创建变更集示例：semifold commit -n describe-change -p qq-cli=patch --tag chore -m "说明变更"。
影响发布内容、产品行为或使用方式的代码/配置变更需要 changeset，并与源文件一起提交。
用户文档（README、使用/开发指南、公开技术方案）使用 patch:chore；纯 Agent 上下文不创建 changeset。
混合提交记录用户可见的变化；历史已清除条目不回填。git commit 与 semifold commit 是不同操作。

## 发布边界

`quality.yaml`在面向main的PR、手动触发及被CD调用时分别运行Windows和原生Apple Silicon质量检查及CLI打包。macOS使用`macos-15` runner并显式要求arm64，运行相同`check-macos.sh`入口。两种平台的产物按提交SHA命名并保留7天。`semifold-status.yaml`报告同仓PR的发布计划；fork PR只运行无写权限质量检查。

`semifold-ci.yaml`在main推送后先复用相同质量门禁，再运行`semifold ci`。Semifold负责release分支、版本PR、changelog，以及版本PR合并后的tag/GitHub Release和资产上传。版本以Cargo.toml为唯一来源，post-version使用`cargo update --workspace --offline`仅同步workspace锁记录。所有setup-semifold沿默认latest策略一致更新，Rust始终读取仓库固定工具链。

`./scripts/package-release.ps1`构建Windows x64 CLI，生成对应ZIP和校验清单；`./scripts/package-macos.sh`生成macOS arm64包。ZIP仅包含原创CLI、README、MIT/Apache-2.0许可证及Skill，打包器检查可执行文件版本、架构、文件范围及哈希，拒绝覆盖已有输出。

发布job下载本次Quality运行的两种平台产物，使用`scripts/release_assets.py`核对它们与Cargo版本一致、原校验清单及归档摘要匹配，再汇总两个ZIP和一份公共`SHA256SUMS`。Semifold的资产列表包含这三个文件，两种安装器据此自动下载并校验。汇总工具使用Python标准库，不执行包内程序、不构建另一份载荷或上传文件。

Semifold通过cargo publish --locked发布crates.io，发布步骤从仓库secret CARGO_REGISTRY_TOKEN读取令牌；令牌应具有qq-cli的发布权限，首次发布还需允许创建该crate。资产在workflow调用Semifold前准备。源码包与ZIP均不包括Agent内部上下文、NapCat、QQ或运行数据。

macOS包也可本地生成（先执行`cargo fetch --locked`）：

```sh
./scripts/package-macos.sh
```

脚本在原生Apple Silicon上构建固定目标并核对Cargo/可执行文件版本；Python标准库打包器验证thin arm64 Mach-O，生成`artifacts/macos/qq-cli-vVERSION-macos-aarch64.zip`及独立`SHA256SUMS`。ZIP只含CLI、README、双许可证和明确白名单内的Skill，固定条目时间并保存Unix执行权限，拒绝链接、未审查的Skill文件和已有输出。可传一个新输出目录参数；不覆盖中断或既有包。`python3 -m unittest discover -s scripts/tests -v`独立验证包边界和反例。

`scripts/install-macos.sh`使用macOS自带Bash、curl、JXA、ditto和校验工具，不要求最终用户安装Python、Git或Rust。默认查询稳定Release，只有唯一同仓macOS资产及校验清单才继续；也可显式传入`--archive ZIP --checksums FILE`离线安装。校验摘要、包条目、原生架构及权限后，在`~/.local/lib/qq-cli/versions/SHA256`保留独立程序，只原子切换`~/.local/bin/qq-cli`入口。自定义目录分别用`--directory`和`--bin-directory`，不得包含符号链接；已有非受管命令拒绝覆盖。安装器不执行下载程序，不写shell配置或Keychain，不清除quarantine；系统ditto可重新生成quarantine时间/来源字段，不能声称逐字节保全。

升级保留前版，重复安装核对原文件不覆盖变化内容；`--uninstall`只退役受管命令链接，保留版本和运行数据。安装锁遗留时先核查原安装进程，不自动删锁重试。合成测试覆盖本地安装/升级/卸载、自动校验、模拟Release发现及双平台资产汇总，不访问真实QQ或凭据。Keychain访问被系统拒绝时由用户显式授权，保留原条目及旧程序；打包流程不要求Developer ID证书，也不执行notarization。

远端启用时，仓库默认分支需为main，并允许Actions创建PR。发布job默认使用GITHUB_TOKEN；如需机器人创建的版本PR自动触发PR工作流，可配置具有本仓contents/PR写权限的GitHub App或PAT为`SEMIFOLD_TOKEN`。未配置时遵守GitHub对GITHUB_TOKEN所发事件的触发限制；版本PR检查可能需要人工批准，也可在release分支手动运行Quality。建议将Windows质量检查列为合并条件；仓库规则仍由维护者配置。

本地检查不执行`semifold ci/version/publish`（包括消费型dry-run）或`cargo publish`。推送和发布遵循维护者授权；运行结果留在内部上下文中。

安装器scripts/install.ps1兼容Windows PowerShell 5.1和PowerShell 7；scripts/check-install.ps1使用合成ZIP和临时目录覆盖安装、更新、校验失败、架构及PATH规则，不修改真实用户PATH。注册表发布和GitHub Release仍由Semifold统一编排，不在本地消费版本或执行发布。
