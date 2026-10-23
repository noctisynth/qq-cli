# macOS 状态与生命周期原锁身份

文档编号：CHECK-081。状态：Native lock integration verified。日期：2026-10-24。
关联：SPEC-008、TASK-006。起点：f200a9a。

## 实现

原实现只保留文件句柄；文件名被替换后，旧句柄锁不再代表当前路径。新增安全Rust Lock持有原Directory、文件和叶名，在等待循环、获取成功及状态事务发布边界复核身份。锁文件NOFOLLOW/NONBLOCK避免跟随符号链接或在特殊文件上阻塞，单链接普通文件和当前有效UID/组其他不可写模式检查先于获取锁。
状态事务从原锁目录发布registry，序列化结束后再核对锁才rename，完成后再次复核；不在失锁后重新选择新目录。生命周期startup.lock复用同一获取能力。Windows原File路径保留；rustix仅增加macOS的process特性用于安全geteuid。

## 验证范围

测试覆盖有界争用/释放、新锁0600、原锁替换导致事务拒绝、根目录替换不接收原事务、符号链接/硬链接/共享可写文件拒绝且不改权限、共享可写根在创建锁前拒绝、独立测试进程观察同一锁及释放。额外测试在序列化期间替换锁，确认原registry不被覆盖。
首次回归发现并发创建锁路径返回ENOENT，附件配额测试的一方因此得到STATE_IO。已将已有文件打开与O_EXCL创建分开，并显式处理创建竞争的EEXIST；原配额测试恢复通过，保留错误码诊断但不输出路径或内容。scripts/check-macos.sh：fmt、Clippy、241项测试与arm64 release构建通过。源码包262项白名单及完整源码覆盖、git diff --check与Semifold status通过；阶段变更集macos-pinned-locks。Windows原生/PowerShell门禁未在本机执行。未验证他人属主的真机样本、ACL、恶意同用户并发删锁防护、全部祖先权限、QQ真实效果边界或完整停止恢复。未触碰待授权Keychain清理。
