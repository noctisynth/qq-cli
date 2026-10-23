# macOS 持久描述符复制工作进程

文档编号：CHECK-097。状态：Synthetic worker and tree integration passed。日期：2026-10-24。
关联：SPEC-008、TASK-006。起点：04eaf0f。

新增安全Rust工作进程模块native/macos/copy/worker.rs、wire.rs，启用rustix的net能力；没有新增unsafe许可或依赖版本。当前CLI私有模式在Clap前分派，仅接收毫秒预算，通过stdin继承匿名UnixStream；环境清空，stdout/stderr丢弃，无来源/目标路径、账号或凭据参数。

SCM_RIGHTS按请求传递恰好两个已打开描述符，固定类型字节区分文件数据/目录或链接元数据/结束。接收端检查截断、数量、类型、目标UID/单链接/空数据文件、不同节点及零共享偏移，收到的OwnedFd设置CLOEXEC并在节点结束后回收。父端每5ms检查deadline和调用方guard；只接受固定回复和固定结束码。独立父连接观察和截止线程可结束阻塞主线程。回收仅针对持有的原Child，最多额外2秒；无法确认退出显式未知，已失败worker不可复用，析构不重复追加清理宽限。

6项工作进程单元测试（其中1项为子进程夹具入口）通过：同一worker连续20个复制请求、来源路径替换仍读取原描述符、正常结束后进程身份失效；忽略请求的子进程在父截止或已发出请求后的guard失效时回收，空目标未被当作成功；异常成功码/同节点能力拒绝；父连接丢失/子截止在主线程阻塞时退出；缺少描述符请求在写入前拒绝。没有用真实QQ进程验证这些用例。

实际CLI私有入口两项集成测试通过：通过SCM_RIGHTS复制原创临时文件、固定成功回复与结束码且标准输出/错误为空；缺失、零、超限预算或额外参数静默拒绝。

新增Inventory.copy_to_new_isolated并接合已有原样副本流程。树内复用一个worker，父端保留路径/目录/guard、quarantine保留、同步、清单核验和最终发布；工作进程必须正常结束且确认回收后才能发布。两项整树合成测试验证包含普通文件/链接的副本指纹及来源重核、超时不发布。同步copy_to_new仍是明确的协作式内核，不作为生产隔离路径；库宿主必须注册私有worker入口，不静默回退。

完整macOS门禁通过：22项Node、fmt、Clippy、299项Rust通过（2项忽略）及arm64 release构建。最后强化请求发送后的guard失效测试，6项worker单元及fmt再次通过，并核对最终Clippy。源码包292项白名单/全部src覆盖、Semifold status及git diff --check通过；阶段变更集macos-copy-worker。未执行Windows/PowerShell原生门禁。

范围：本阶段没有复制、改写、签名或启动真实QQ。隔离覆盖fcopyfile；父端清单、quarantine写入、同步与发布仍是协作式截止，不能据此承诺所有内核调用的绝对墙钟上限。最终ACL/模式加固、父端文件系统预算、安装日志、暂存清理、DMG及启动编排仍待接合。

最终结束请求也先检查子进程截止；实际CLI两项集成测试、fmt、Clippy及arm64 release构建再次通过。仅本地提交，不推送或发布。
