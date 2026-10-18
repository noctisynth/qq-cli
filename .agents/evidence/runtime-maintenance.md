# 实例维护机制与离线验收

文档编号：CHECK-020。状态：停止内核离线验证，公开命令及完整生命周期待验收。更新：2026-10-19。
关联：SPEC-004、RFC-0003、TASK-002、CHECK-019。

## 来源和语义

2026-10-19读取Microsoft Learn可变文档，不作为固定QQ兼容基线：
- [TerminateJobObject](https://learn.microsoft.com/en-us/windows/win32/api/jobapi2/nf-jobapi2-terminatejobobject)：作用当前Job及其嵌套子Job，需TERMINATE权限，进程不能延期或自行处理终止。
- [Job Objects](https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects)：默认CreateProcess子进程继承Job；特定创建机制/逃逸配置不保证继承，不能据此宣称真实QQ完整进程覆盖已验收。
- [Job Object Security](https://learn.microsoft.com/en-us/windows/win32/procthread/job-object-security-and-access-rights)：QUERY与TERMINATE为不同权限；创建返回全权限，重开按对象DACL检查权限。

因此查询/业务能力保持QUERY，显式维护才取得不继承的QUERY|TERMINATE，并且只能以已验证签名运行记录的原随机Job定位。强制结束共享QQ会话不等于QQ登出或保存草稿；只开发并实验原创子程序，不操作真实QQ。

已有命名Job实验证明关闭最后用户句柄后，不能以名称丢失推断成员均已退出。维护必须在持句柄时观察空组、持久完成证明后才登记stopped；意图完成但登记提交失败可依证明恢复。若中断后原Job不可重开且无证明，保留未知，不制造已停止状态；此处不声称所有崩溃都能自动恢复为确定结果。

## 停止内核增量

实现独立停止编排、签名p1证明和原Job维护能力；启动/停止共用生命周期锁，状态事务不跨原生等待。只有原j1成功调度记录可定位Job；没有调度认领的准备态可以取消。预启动取消中断于stopping后，可由同目标原签名意图恢复；完成证明先于登记确认，重复完成不新增证明。安装摘要变化、错误密钥、选择后的run变化、未签名stopping及被回写为prepared的已有启动认领均拒绝。

新增7项库级测试：6项临时原创状态恢复/拒绝测试，1项原创子进程Job权限和隔离测试。后者证明QUERY不能终止、关闭维护句柄不终止、过期调用无终止作用、显式终止原组不影响另一组。库级58项通过；受限运行最初因私有目录ACL检查失败，在原生用户权限下重跑通过，没有放宽产品ACL。

原生实验还观察到Job计数可先为空、进程对象随后才报告PROCESS_EXITED；测试分别有界等待这两个观察，不宣称空组即所有进程句柄同步完成退出。未进行真实QQ进程覆盖或窗口退出验证。

公开stop路由/Schema、真实运行记录接合及完整故障矩阵仍待验收；remove/restart/更新回滚没有因本内核完成而提升状态。业务清单仍60/93。

本增量完整离线回归238项通过，fmt、全目标clippy、release构建、架构边界及180文件源码包白名单通过；Semifold记录41条未消费changeset，仍为单发布单元，不执行version/publish。
