# Apple Silicon 离线开发基线

文档编号：CHECK-073。状态：Offline verified。日期：2026-10-24。
关联：TASK-006、SPEC-008。仅原创测试，不启动 QQ/NapCat、不读写真实会话。

## 变更和方法

以 d501701 为起点，在原生 Apple Silicon macOS / 固定 Rust 1.98.1 上复核。
受限沙箱最初同时拒绝回环监听和带系统别名的临时路径；在允许合成服务监听并解析真实临时目录后，库测试复现 37/40，三个 HTTPS 正例失败。
旧静态证书有效期为 2020–2045；改为 rcgen 0.14.5 在进程内生成独立 CA 和短期叶证书后，TLS 正例及原有不可信根/错误主机名负例全部通过。
没有禁用证书/主机名校验，没有修改系统信任库；动态证书也避免夹具随日历过期。此对照没有单独穷举原证书失败的所有系统策略原因。

新增 scripts/check-macos.sh，要求 Darwin/arm64、解析 TMPDIR 系统别名，依次运行 fmt、Clippy、全部可编译测试及 aarch64-apple-darwin release 构建，Cargo 均离线且锁定依赖。
未改变私有状态拒绝符号链接的行为。Windows 独有运行调用对应的未使用项改为精确条件编译，不添加全局警告豁免。
安装 CLI 的集成断言明确区分当前平台凭据能力：Windows 仍要求缺文件错误，macOS 当前返回 PLATFORM_UNSUPPORTED；不据此勾选 macOS 安装或凭据实现。

## 结果与限制

- fmt 与全目标 Clippy（警告视为错误）通过。
- 206 项 macOS 可编译测试通过，0 失败、0 ignored；Windows cfg 排除的测试不计为通过。
- 原生 arm64 release 构建通过。
- 源码包白名单检查通过，243 项；所有 Rust 源文件包含，内部上下文与私人材料不包含。
- Semifold 配置同步及 status 检查通过；阶段变更集 macos-test-baseline。

Windows 原生测试、PowerShell 门禁及 Windows TLS 回归须另行运行。本记录不证明 macOS 进程所有权、凭据、安装/启动或 CLI 真实功能已实现。
