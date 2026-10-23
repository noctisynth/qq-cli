# Windows 原生目录隔离与非 Windows 编译边界

文档编号：CHECK-067。状态：目录/编译切片已验证，完整平台验收未完成。日期：2026-10-23。
关联：RFC-0008、SPEC-004、TASK-006、FACT-005。

## 变更

12 个 Job/credential_lock 实现与测试文件原样移入 native/windows，后者入口改用 credential_lock/mod.rs 以保持子模块解析。native/mod.rs 使用 cfg(windows)/path 选择；逻辑模块名、测试过滤路径、unsafe 两处授权、签名记录与行为不变。结构门禁新增精确平台路径检查。

修复 app/runtime 无条件导入 Windows Started 导致的 macOS 编译错误：导入与专用 response 同步条件编译，平台分支的未使用导入/变量就地处理。通用 CLI 测试只对非 Windows 的明确 remove 用例接受既有 PLATFORM_UNSUPPORTED，仍验证不创建状态，不扩大其他命令的错误集合。

## 验证

- 迁移的 12 文件与 HEAD 原位置逐字节一致；未修改原生算法、序列化与运行日志。
- Rust 1.98.1 固定工具链 fmt 通过。
- macOS cargo check --all-targets --offline --locked 通过；仍有 Windows 专用辅助项的 dead_code 警告，不能称 macOS 严格 Clippy 全通过。
- x86_64-pc-windows-msvc 的 cargo check --lib 及离线 Clippy -D warnings 通过。仅交叉检查生产库，未运行 Windows 原生测试或验证 Windows 链接。
- PowerShell 未安装：结构规则以独立 Python 核对通过，包括反向依赖、根入口、unsafe 范围及新增平台路径；未执行 check-architecture.ps1 本身，不冒称 Windows 完整门禁通过。
- macOS 库测试：首次受沙箱回环限制与默认临时目录符号链接影响。规范临时目录及允许原创回环后，40 项中 37 项通过、3 项 HTTPS 测试失败。
- 临时原创 TLS 诊断（完成后恢复源文件）确认三项失败的共同连接错误为 macOS 原生 TLS -67901：证书有效期超过允许上限。现有 synthetic-server-chain.pem 为 2020-01-01 至 2045-01-01；尚未改证书夹具，不关闭证书验证或扩大系统信任。
- 通用 CLI 初次 13/14 通过：缺失实例 remove 在非 Windows 返回既有 PLATFORM_UNSUPPORTED，原测试只接受 Windows 路径错误；补精确平台预期后 14/14 通过，包含全部帮助入口与 Schema 验证。

## 后续

下一切片处理可移植测试夹具与平台未使用代码，再拆分身份/发现/凭据及生命周期。TLS 夹具应采用适合目标平台的有效期和可维护生成策略，避免硬编码即将过期的日期；不能通过关闭验证让测试通过。
Windows 原生回归、PowerShell 门禁与 macOS 真实 QQ 运行仍未完成；本次未启动/停止/修改 QQ、未连接真实 OneBot、未推送或发布。macOS 原版 Framework 加载路径仍未确认。
