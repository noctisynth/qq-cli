# 贡献指南

文档编号：CONTRIB-001。状态：Active。更新：2026-10-22。

## 上下文

从 [AGENTS.md](AGENTS.md) 读取协作规则，再按 [.agents/DESIGN.md](.agents/DESIGN.md) 定位相关需求和技术方案，当前任务见 [.agents/TODO.md](.agents/TODO.md)。工程上下文不随源码包或 CLI 压缩包发布。

修改Rust代码先读内部[架构与维护入口](.agents/architecture.md)。源码按app/cli/core/storage/requests/messaging/napcat/native/transport组织；跨层编排、原生边界和测试归属遵循该文档。RFC-0005/TASK-003记录已完成的质量整改。

## 当前工具

需要 Git、Semifold、rustup 及 Windows MSVC 链接工具；Rust 工具链由 rust-toolchain.toml 固定为 1.98.1。
Cargo.toml 是唯一包版本来源，Semifold 使用 rust 解析器，旧 package.json 已移除。
Cargo.lock 锁定依赖；HTTP 仅接入已登记的字面回环地址，禁用代理、重定向和自动重试。
现有实验探针不是 CLI 产品代码，不作为正式安装或使用入口。

首次准备可联网执行 cargo fetch --locked；随后运行以下离线检查：

```powershell
./scripts/check.ps1
```

脚本依次执行架构/契约边界、合成发布包、安装器与版本锁同步检查、fmt、clippy（警告视为错误）、全部串行单元/集成测试、release 构建和源码包白名单检查；构建/测试/包清单使用 --offline --locked。
测试使用合成数据、临时私有目录、内存后端和有截止的回环假服务，不需要运行 QQ/NapCat，也不读取真实配置或聊天。
Windows 私有 ACL 和回环监听检查需要普通用户的对应系统权限；受限沙箱可能拒绝，不能通过削弱权限或跳过测试代替验证。
核心消息及实例管理已有正常功能证据；完整进度及剩余Agent/交付验收见TASK-002，不以离线检查通过替代真实会话或稳定性验收。

Apple Silicon macOS 开发检查（先执行 `cargo fetch --locked`）：

```sh
./scripts/check-macos.sh
```

该入口运行原生 arm64 的 fmt、Clippy、全部可编译测试及 release 构建，自动解析临时目录的系统别名，保持私有状态拒绝符号链接的规则。测试需要回环监听及创建原创子进程的权限，不访问 QQ。TLS 测试为每次运行生成短期证书，仅向合成客户端添加信任，不修改系统证书库。
这不是 macOS 运行支持或 Windows 门禁验收；Windows 原生、安装器和发布包检查仍执行 `check.ps1`。

常规检查：git diff --check、semifold config sync --check、semifold status、git status --short。
创建变更集示例：semifold commit -n describe-change -p qq-cli=patch --tag chore -m "说明变更"。
影响发布内容、产品行为或使用方式的代码/配置变更需要 changeset，并与源文件一起提交。
用户文档（README、使用/开发指南、公开技术方案）使用 patch:chore；纯 Agent 上下文不创建 changeset。
混合提交记录用户可见的变化；历史已清除条目不回填。git commit 与 semifold commit 是不同操作。

## 发布边界

`quality.yaml`在面向main的PR、手动触发及被CD调用时运行Windows质量检查和自产CLI打包。`semifold-status.yaml`报告同仓PR的发布计划；fork PR只运行无写权限质量检查。

`semifold-ci.yaml`在main推送后先复用相同质量门禁，再运行`semifold ci`。Semifold负责release分支、版本PR、changelog，以及版本PR合并后的tag/GitHub Release和资产上传。版本以Cargo.toml为唯一来源，post-version使用`cargo update --workspace --offline`仅同步workspace锁记录。所有setup-semifold沿默认latest策略一致更新，Rust始终读取仓库固定工具链。

发布资产通过`./scripts/package-release.ps1`构建Windows x64 CLI，生成`artifacts/qq-cli-vVERSION-windows-x86_64.zip`和`SHA256SUMS`。ZIP仅包含原创exe、README、MIT/Apache-2.0许可证及Skill，构建后核对exe版本、PE架构、文件范围及哈希；已存在输出会拒绝覆盖。Semifold通过cargo publish --locked发布crates.io，发布步骤从仓库secret CARGO_REGISTRY_TOKEN读取令牌；令牌应具有qq-cli的发布权限，首次发布还需允许创建该crate。资产在workflow调用Semifold前准备。源码包与ZIP均不包括Agent内部上下文、NapCat、QQ或运行数据。

远端启用时，仓库默认分支需为main，并允许Actions创建PR。发布job默认使用GITHUB_TOKEN；如需机器人创建的版本PR自动触发PR工作流，可配置具有本仓contents/PR写权限的GitHub App或PAT为`SEMIFOLD_TOKEN`。未配置时遵守GitHub对GITHUB_TOKEN所发事件的触发限制；版本PR检查可能需要人工批准，也可在release分支手动运行Quality。建议将Windows质量检查列为合并条件；仓库规则仍由维护者配置。

本地验证不执行`semifold ci/version/publish`（包括消费型dry-run）或`cargo publish`；不推送、不触发GitHub Release。workflow语法和本地构建通过不等于GitHub托管runner及远端发布已实跑，部署与交付验收见TASK-005。原版Framework的独立实验也不替代CLI验收。

安装器scripts/install.ps1兼容Windows PowerShell 5.1和PowerShell 7；scripts/check-install.ps1使用合成ZIP和临时目录覆盖安装、更新、校验失败、架构及PATH规则，不修改真实用户PATH。注册表发布和GitHub Release仍由Semifold统一编排，不在本地消费版本或执行发布。
