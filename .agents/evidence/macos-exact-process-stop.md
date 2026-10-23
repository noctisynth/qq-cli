# Apple Silicon 原进程代次定向终止

文档编号：CHECK-076。状态：Native synthetic verified。日期：2026-10-24。
关联：SPEC-008、DESIGN-002、TASK-006。起点：5ca74c7。

## 机制与来源

核对本机 Apple SDK 的 libproc.h、mach/task_info.h：task_name_for_pid 提供只读 task-name right，TASK_AUDIT_TOKEN 返回8个32位字段；proc_signal_with_audittoken 在内核核对原目标代次。
原创 terminate.c 仅做 SDK 类型转换、task-name right 释放和固定 SIGKILL 派发，不使用 task_for_pid、调试/task-control 权限或数字 PID 回退。
私有 Target 保存版本、原 Identity 与原 audit token；前后核对 Pinned，恢复时重新核对 token，不刷新陈旧权威。它可供后续签名日志使用，当前未接合 CLI 停止编排。

## 验证

- 初始独立原创子进程探针：取得只读 task-name/token 成功，错误代次返回 ESRCH，正确代次信号被接受。未输出 token 字段。
- 5项新增测试（含两个原创子进程入口）：Target序列化/恢复、正确派发与原观察退出确认、重复调用拒绝；错误子进程/过期/损坏token/未知版本拒绝且双方存活；同PID执行新程序后应用守卫拒绝。
- 同PID exec 测试还直接将旧token交内核，确认内核拒绝且新程序存活，避免仅用应用层比较冒充代次保护。目标始终为测试创建且尚未回收的子进程。
- scripts/check-macos.sh：fmt、Clippy、224项测试及arm64 release构建通过。
- 源码包255项白名单与完整源码检查通过。五处原生unsafe声明、macOS精确条件编译正则及git diff检查通过；本机未运行Windows/PowerShell原生回归。
- Semifold status通过，阶段变更集macos-exact-process-stop。

## 边界

信号接受不等于退出确认，单进程退出不等于运行进程树停止。上层仍须持久一次性停止意图、验证完整成员归属、观察原进程退出、保存完成凭据；不能仅凭PID重建Target。
本阶段不启动/停止QQ，不验证QQ本身的task-name权限、父退出/脱离子进程覆盖、跨CLI停止恢复或最低macOS版本。没有把本测试替代整组stop验收。
