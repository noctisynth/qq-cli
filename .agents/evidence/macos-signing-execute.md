# macOS 受控副本签名内核

文档编号：CHECK-105。日期：2026-10-24。状态：合成端到端、完整macOS门禁及最终定向复核通过。
关联：SPEC-008、TASK-006。起点：d401bd4。

原创C载荷探测确认codesign --entitlements -不读stdin，管道形式的/dev/stdin和/dev/fd/0均失败（cannot read entitlement data）；取消目录链接的普通文件作为stdin、通过/dev/fd/0读取成功。仅签名原创合成代码，不执行载荷、不接触真实QQ。

Inventory.sign_copy拒绝普通inspect生成的对象，仅消费活跃复制能力；复核原父目录与guard、加固权限/UID/ACL、原计划及所有原封印。权益以原父FD独占创建0600文件，先unlink再写固定Policy模板，确认零链接/UID/模式/无ACL后传入固定codesign；无shell、环境继承、动态参数或生产--deep。计划内到外执行，当前整树stamp在效果前复核，逐目标完成后整树扫描、允许变化与原载荷比较、Policy验证；最后再次验证全部角色与整树，才返还能力。

合成端到端包括原创测试程序app/helper、C Framework及native叶：普通扫描签名拒绝；复制到私有父目录、加固、全计划签名、逐目标系统权益验证及源树保全通过，父目录仅留下预期副本，没有权益临时文件。直接Child等待另测成功/失败退出、已截止和guard失效，有界终止与原进程退出观察；工具退出成功不独立授予成功结果。

失败消费能力、保留部分副本，不回滚或自动重试。wait不确定后不发信号，无法确认回收为SIGNING_CLEANUP_UNKNOWN，Drop不重复延长回收窗口。此内核不覆盖调用进程突然退出后的codesign监管、阻塞系统签名验证隔离、磁盘同步/恢复日志或大型应用逐目标扫描预算。生产启动仍不可直接调用并宣称完整交付，相关待办保留。Windows原生/PowerShell门禁未执行。无真实QQ、账号或Keychain操作，仅本地提交，不推送/发布。

完整门禁通过：22项Node、fmt、Clippy、319项Rust通过（2项忽略）及arm64 release构建。门禁测试编译后补入退出后截止重核，最终源码另跑fmt、Clippy及2项直接Child定向测试通过，release使用最终源码。定向测试首次在沙箱内因PROCESS_UNVERIFIABLE失败，以完整门禁相同权限重跑成功；不将环境拒绝隐去。源码包304项全部源码覆盖、内部/私人材料排除、证据唯一编号/相对链接、git diff --check及Semifold status通过。阶段变更集macos-signing-execute。
