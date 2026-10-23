# Apple Silicon 进程身份与退出观察

文档编号：CHECK-074。状态：Native synthetic verified。日期：2026-10-24。
关联：SPEC-008、DESIGN-002、TASK-006。起点：7872c2a。

## 实现与来源

native/process/identity.rs 保持安全 Rust 的私有平台编码；Windows 原 processId/created/image 不变，旧记录不重签，不把 FILETIME 解释为 macOS 时间。
macOS 独立编码绑定 platform、bootSession、PID、秒/微秒启动时间和映像。字段缺失、混入 Windows created、未知平台及额外字段拒绝；跨平台 reopen 在原生查询前拒绝。
原生边界限 native/macos/process.rs，使用锁定 libc 0.2.191 的 proc_pidinfo/proc_pidpath、sysctlbyname 和 kqueue。
接口与结构核对本机 Apple SDK 的 sys/proc_info.h、libproc.h、sys/event.h；这里只记录接口来源，不把本机 SDK 推定为最低系统版本支持证明。

先观测指定 PID，再注册 NOTE_EXIT/NOTE_EXEC 并复核；映像读取前后核对启动时间。kqueue 的原注册在退出/exec 后永久失效，不自动重开 PID。描述符不可继承，释放观察能力不终止目标。错误不输出映像或启动标识。

## 验证

- 三项身份测试：Windows 旧编码逐字序列化保持、平台互斥与必填字段、非法 macOS 身份在查询前拒绝。
- 三项原生集成测试：原创子进程完整身份与重开、身份三个字段不匹配、退出后连续拒绝；子进程 exec 到另一程序且 PID 仍存活时连续拒绝；非法/过期/跨平台请求拒绝。
- exec 测试使用显式通知及五秒接收截止；最初无截止的握手测试被终止并修正，没有把超时当作成功。
- scripts/check-macos.sh：fmt、Clippy、212 项测试、arm64 release 构建通过；最后测试文本调整后重新运行身份定向测试和 Clippy。
- 独立检查 unsafe 仅在三处原生能力声明放行；同步 Windows PowerShell 结构门禁，但本机未执行 PowerShell 或 Windows 原生回归。
- Semifold 阶段变更集：macos-process-identity。

## 限制

只观测测试自己的子进程，不启动、停止或读取 QQ。本阶段不提供同 socket 归属、进程树所有权或终止权限，不证明 PID 实际回绕/重用压力、父退出或子进程脱离已覆盖。
通用运行编排与 j1/t1 签名日志仍待平台化；macOS CLI 尚不能据此启动真实实例。原生调用前后检查总截止，不能声称可强制取消单次系统调用。
