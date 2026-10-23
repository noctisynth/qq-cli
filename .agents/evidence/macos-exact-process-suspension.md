# macOS 原进程代次暂停与恢复

文档编号：CHECK-078。状态：Native synthetic verified。日期：2026-10-24。
关联：SPEC-008、TASK-006。起点：4779260。

## 机制

沿用CHECK-076的原audit token内核派发，新增固定SIGSTOP/SIGCONT入口，不提供任意信号或数字PID回退。每次派发前核对原Pinned和audit token；只调用一次，接受不等于状态完成。SSTOP经原身份前后复核观察。
暂停后的Target仍可序列化/重开，供后续签名意图恢复。Drop不恢复目标；生命周期必须保存暂停归属，不能恢复用户先前暂停的目标。

## 验证

六项定向测试通过：原暂停被观察后，输入可用而子进程不继续执行；原Target重开后显式恢复，子进程消费输入并退出。错误目标、未知版本、过期请求均拒绝；同PID exec之后，应用守卫和直接原token内核调用都拒绝暂停/恢复替代程序。
另以本地原创C探针核对sandbox执行白名单继承：fork/setsid后的子进程可执行白名单内原程序，范围外系统true被拒绝。该探针仅有执行限制，并非生产隔离profile，不证明GUI/Mach服务限制或间接启动隔离。
scripts/check-macos.sh：fmt、Clippy、227项测试及arm64 release构建通过。源码包257项白名单及源码覆盖、git diff --check、Semifold status通过；变更集macos-exact-process-suspension。Windows/PowerShell原生回归未执行。本阶段没有操作QQ、未验证跨CLI持久恢复或进程树停止。

## 下一步约束

逐文件动态候选快照可能漏过切换映像的进程，不能用空结果证明完整树停止；暂停单个目标同样不证明全部成员暂停。完整生命周期需独立的执行范围与覆盖证明、持久暂停/停止意图、失败清理和退出确认。用户以外的控制者仍可能恢复目标，SSTOP不是排他内核锁。

后续源码线索（2026-10-24读取）：Apple公开 [XNU proc_info.c](https://github.com/apple-oss-distributions/xnu/blob/main/bsd/kern/proc_info.c) 中 proc_listpids 在 proc_list_lock 内收集PID，支持按UID过滤；这是可变main来源，未固定提交，也不证明当前运行内核版本相同。可用于研究单次候选PID集合后逐项分类/冻结的覆盖算法，不能直接作为完整停止已验证的依据。上游源文件仅存本地临时目录。
