# Apple Silicon Framework 运行适配方案

文档编号：SPEC-008。状态：接入原型已验证；Rust 平台实现待完成。更新：2026-10-24。
需求：RFC-0008。事实：FACT-005；验证：CHECK-072；任务：TASK-006。

## 装配与签名

以固定官方 DMG 建立工具拥有的专用 QQ 副本，保留原安装；不复制真实 QQ 私人数据。
副本 package.json.main 指向外部薄适配入口，继续原 QQ application.asar/app_launcher/index.js。
只对专用副本本地 ad-hoc 重签名。已验证的原生 hook 需要 allow-jit、disable-library-validation，
主进程另需 allow-unsigned-executable-memory；不加入 disable-executable-page-protection，不关闭 Gatekeeper/SIP。
这是验证组合的配置，不是其它 QQ/系统版本的通用白名单；生产安装需重复验证来源、完整性和权限范围。

移除副本的 App Sandbox/共享组权益后，实验使用外部 sandbox 强制目录与进程隔离。
产品必须实现并验证等价的所有权和隔离边界，不能因副本可运行就省略隔离；实验脚本不直接充当安装器。

## 真实会话转交

适配在官方入口前安装精确 wrapper.node 注册观察，仅处理副本的固定模块路径，其他模块原样转发。
原 QQ 自己调用 getNTWrapperSession("nt_1") 时，向原版 nativeLoader.cjs 的 initializeNapCat 转交取得的原生对象和登录服务。
不调用不存在的 session.create，不创建另一 session，不运行 Shell 初始化。

原生 init/getter 属性不可写；通过可替换的模块导出和转发视图接入，保留原 receiver、参数、返回、异常和其它方法。
观察官方 init 的真实 onOpentelemetryInit，先执行原回调，再在 is_init === true 且代次匹配时释放 Framework 初始回调。
先注册/后通知或先通知/后注册都必须正确；拒绝过期代次及 truthy 非布尔成功，初始就绪只释放一次。
该原型不是切号/重连状态机；后续恢复需要独立设计和验收。

当原生 getNTUserDataInfoConfig 返回空值时，外部适配回退到受控 userData；非空原生结果原样保留。
每次精确模块注册均应用该兼容，不改上游文件；生产使用前仍须检查最终数据路径属于实例。

## 实例目录和端点

使用结构正确的私有容器布局：<instance>/profile-home/Library/Containers/com.tencent.qq/Data。
QQ 由 home 推导的 Group Containers 必须仍落在同一实例根下。
Electron home/appData/userData、documents/downloads/desktop/media、temp/logs 逐项显式设置到受控目录。
不能用扩大到原用户目录的读写权限解决路径失败。符号链接/祖先解析、旧路径迁移和并发写入需产品级检查。

原版默认配置名为 onebot11.json，账号配置为 onebot11_<account>.json；不能写成 onebot.json。
新实例不预填账号，不覆盖已有账号配置。业务 HTTP/WS、管理端点及凭据分离，回环监听，关闭非需求插件/CORS/调试。
上游日志留私有目录；不能用日志中的 token/账号/二维码作为公共状态输出。

## Rust 平台接口

通用编排依赖平台能力：准备启动、验证进程身份、验证同一连接归属、观察退出、停止已拥有运行。
macOS 身份不能复用 Windows FILETIME/Job 的编码；平台标记、旧记录读取/拒绝和迁移策略先定义再实现。
libproc、kqueue、Keychain、进程组或保管机制是候选，尚未因原型实验被证明足以满足所有权契约。
要覆盖 PID 重用、父退出、子进程脱离、端口转移、凭据恢复、停止前重核及失败清理。

保留同一 socket 的归属核验后再发送认证，不能把一次 lsof 监听查询或 URL/token 当作产品受检连接。
安装发现、DMG/app 处理、签名、兼容诊断与生命周期接合后，再复用 OneBot/领域层并补 arm64 CI/安装分发。

## 进程身份首阶段

Identity 使用平台互斥的私有编码：旧 Windows processId/created/image 格式继续按 FILETIME 读取和原样序列化，不迁移或重签旧日志。macOS 编码必须包含 platform=macos、bootSession、processId、startSeconds、startMicroseconds、image；不将 FILETIME 换算为 Unix 时间，不允许跨平台恢复。
macOS 使用 libproc 只查询明确 PID 的 BSD 启动时间和映像路径，启动时间保留秒/微秒，bootSession 绑定本次系统启动。每次观测两侧复核启动身份，拒绝退出中/僵尸进程。
以独立 kqueue 的 EVFILT_PROC 注册 NOTE_EXIT/NOTE_EXEC，注册前后核对身份；后续观察只使用原队列，退出或 exec 事件永久使该能力失效，不按 PID 接管替代进程。队列句柄不继承，Drop 仅关闭队列，不结束目标。
重开持久记录需要重新注册观察并核对完整身份；没有原记录不能取得运行所有权。这个只读进程能力不证明进程树、同 socket 归属、实际加载摘要或终止权限，不替代后续运行隔离与受检传输。
macOS 原生 unsafe 只开放在 native::macos_process（物理路径 native/macos/process.rs），每个 FFI 标注缓冲区及句柄不变量；通用身份与编排不开放 unsafe。

## 已连接 socket 归属

仅查询已有 Pinned 指定的服务进程描述符，不枚举全系统进程或读 socket 负载。PROC_PIDLISTFDS 限8192项，PROC_PIDFDSOCKETINFO 只接受完整结构与 TCP ESTABLISHED；监听或尚未 accept 的连接不能代替服务端已连接描述符。
原生 SDK 的 socket union 由原创窄 C 适配读取，编译时使用平台头文件，输出仅族、地址、端口、socket标识与代次；Rust FFI 仅在 native::macos_socket 放行，不手写系统结构偏移，不输出或持久化其它描述符信息。
四元组须与原 TcpStream 的 local/peer 反向完全相等，限 IPv4/IPv6 字面回环、无 scope/flow。唯一匹配后，以原描述符重查同 socket标识/代次并再次复核进程；失败关闭原流，不重连或发送协议数据。
服务端 accept 可异步完成，仅“尚无匹配描述符”允许在命令截止内等待，最多2秒/200次、每次最多10ms；查询错误、重复匹配和身份变化立即拒绝。进程仍可能主动转移 socket，本检查不是禁止转移的内核能力；不能据此单独授予运行停止或接管权限。
OneBot/WebUI 受检 HTTP 和 WS 复用通用协议实现，经平台 guard 验证原 socket 后才写认证，响应/握手后再次复核。macOS guard 输入必须由后续签名运行状态提供；单独构造 Pinned 不是 CLI 自动接入许可。

## 定向终止能力

macOS 使用公开 libproc 的 proc_signal_with_audittoken，终止固定发送 SIGKILL，停止收敛可分别派发 SIGSTOP/SIGCONT；不使用只按数字 PID 的 kill/killpg 作为持久运行恢复后的终止依据。
仅从已验证 Pinned 取得只读 task-name port，查询 TASK_AUDIT_TOKEN 后立即释放 Mach port，前后复核原进程身份；不获取调试或 task-control 权限。原 token 与完整 Identity 保存在私有可签名 Target 中，不提供 Debug 或公共输出。
恢复 Target 时先核对平台/启动身份，再重新读取 token 并精确比较。终止前再次复核当前 token，内核调用仍携带原 token，使 PID/exec 代次变化不能终止替代进程。没有有效原 Target 的运行不能靠 PID 重建终止权威。
原生 signal 成功只表示内核接受一次信号，不表示整组已退出；确认必须观察原 Pinned 的退出事件。任何查询、身份、权限或内核拒绝均明确失败，不重试。上层须先保存一次性停止意图，并独立完成进程树所有权、成员范围及持久完成凭据；单进程能力不能勾选完整 stop。
暂停与恢复同样只向原 audit token 各派发一次，不开放任意信号接口。信号接受与 SSTOP 观察分离；观察前后复核原 Pinned。SSTOP 是瞬时内核状态，不是排他的冻结锁，也不证明所有子进程已暂停。生命周期须先持久暂停意图与恢复目标，不能自动恢复用户原本已暂停的进程；Drop 不派发恢复信号。进程已暂停仍可重开原 Target，供独立恢复流程使用。
FFI 限 native::macos_terminate 与原创 terminate.c，使用 SDK audit_token_t/端口类型；请求 token 仅在私有内存/签名状态传递，不记录系统用户/会话字段。

## 专用可执行文件候选发现

原生 Executable 保留规范绝对路径、打开的文件描述符及设备/inode/大小/权限/属主/时间戳，拒绝符号链接路径、硬链接与非可执行普通文件。查询前后复核路径和原文件元数据；这不是文件内容摘要或加载映像完整性证明。
原创 SDK 适配以 proc_listpidspath 的精确文件模式查询，最多4096项；缓冲区满、异常PID、重复项或任一候选不可观察均拒绝本次结果。系统返回的是文件引用，必须再以 Pinned 的实际 image 路径过滤，排除只打开文件的其它程序。
结果仅为动态候选快照，不授予所有权、停止权限或“整树为空”的完成凭据。父退出/更换进程组不影响文件查询，但两次扫描间仍可 fork/exec；实际映像内容也不能仅凭路径认定。后续运行库存须独立认证文件与启动来源，限制可执行范围，并实现成员冻结/终止收敛和持久停止意图；未完成这些条件前不接入 CLI stop。
FFI 仍限原有 macos_process 边界，census.rs/census.c 不新增终止能力或全局进程元数据输出。

## macOS 凭据适配

SystemCredentials 使用固定 security-framework 3.7.0 的安全 Rust API访问本机非云同步 Generic Password，service 继续由规范化状态目录摘要派生，account为已有逻辑凭据键。macOS 不套用Windows目录重定向的legacy namespace回退；空键、控制字符和过长键在接触Keychain前拒绝。
读取、创建/更新和删除分别检查系统返回值；只有errSecItemNotFound映射为缺失或幂等删除。访问拒绝、锁定及需交互等错误明确失败，不回退明文、不替换已有签名权威。
Keychain交互开关属于进程全局状态；本适配用进程内Mutex串行保护，操作期间禁用系统交互，只在原先允许交互时创建恢复guard。该Mutex不代替跨CLI状态/生命周期锁，原签名密钥初始化仍依赖Store事务。
Keychain条目存在创建程序访问身份：同一可执行文件的创建/读取/更新/删除及新进程读取/清理通过，不代表不同签名程序或升级后的CLI可以无交互访问。现阶段不同测试程序清理由CLI创建的条目被拒绝，正式发布签名、升级后的授权恢复与跨CLI验收仍须完成。不得把这个适配提升为完整持久恢复能力。

## macOS 状态发布与落盘

atomic_json在macOS arm64通过锁定rustix 1.1.5安全API逐层打开目录，要求每层为非符号链接目录；后续创建与renameat相对于保留的目录描述符。拒绝包含父级跳转的路径、已有非普通文件或多硬链接目标。临时文件独占创建且模式0600，序列化失败清理临时文件并保留旧目标。
发布顺序为写入完整JSON、F_FULLFSYNC临时文件、复核目录身份、同目录renameat、fsync目录、再次F_FULLFSYNC发布文件并复核目录身份。重命名后同步失败仍返回错误，不把可见文件当成成功凭据；调用方不得因保存失败继续真实操作。
References签名材料使用同一固定目录句柄独占创建0600文件，目录同步及F_FULLFSYNC完成后才返回引用。新增目录逐层mkdirat(0700)，不修改已有父目录权限；每次新增目录同步其父目录项，之后文件发布的设备flush覆盖此顺序。
这是写入顺序与路径变化检查，不是断电实测或任意文件系统保证。现有目录的属主/ACL/权限验证、锁文件稳定身份、完整读取路径约束和同用户并发修改防护仍须单独完成。Windows原有实现保持。

## macOS 状态与生命周期锁

state.lock与startup.lock通过同一已固定目录描述符openat，先打开已有文件；缺失时独占创建，遇到另一进程先创建的EEXIST则重新打开并复核。使用NOFOLLOW/NONBLOCK/CLOEXEC、创建模式0600，不截断既有内容。锁目录须属于当前有效UID且POSIX模式不允许组/其他用户写入；锁文件须为当前UID拥有的单链接普通文件，且不允许组/其他用户写入。已有0644锁文件可继续使用，不自动chmod或接管其它用户文件。
等待之前、每次重试及成功取得锁之后，复核目录路径对应原设备/inode、锁文件路径对应原文件、属主/模式/链接数。竞争沿用STATE_BUSY/START_BUSY且受截止约束；Drop只关闭句柄，不删除锁文件。该检查能发现原锁被替换，不是对恶意同用户删锁的排他防护。
状态事务读取后再次复核锁，持久化使用原锁目录句柄；序列化后、rename之前再次复核锁，发布后仍复核。丢失原锁不得作为成功提交。生命周期调用已使用相同获取逻辑，后续macOS真实进程操作还须在效果边界复核能力，不能仅凭取得过锁就授予运行权威。
start_intent的真实效果回调之前重新核对原状态锁，失败则不调用回调。回调之后、回执保存前后再次核对；此时失锁返回START_OUTCOME_UNKNOWN并附原request_id，不宣称可安全重试。回调后的第一次重核失败时不尝试写回执；原Pending意图保留，恢复时只转Unknown，不重新发放启动机会。该检查覆盖现有通用dispatch入口，不代表macOS受控启动与完整日志格式已经接合。
上述是POSIX属主/写权限检查；ACL、其它祖先目录的可写策略、全体读取路径与完整运行恢复仍需单独验收。

## macOS 服务记录的只读恢复

macOS服务绑定使用独立macos-runs索引和j2签名引用，不读取、迁移或重签Windows j1/旧t1记录。索引按(instance_id,run_id)摘要寻址，pointer schemaVersion=1；签名正文schemaVersion=1且platform只能为macos，包含原requestId、完整start_intent Target、原服务进程Identity、固定HTTP与可选WS端点。
恢复必须持有原状态锁，要求当前注册表中实例与安装分别唯一且与调用快照相同，实例处于starting/login_required/ready，安装摘要、运行代次和两个端点与签名记录完全匹配。原s1启动回执须为Dispatched并具有同一完整Target，包括inputHash；Pending/Unknown不能恢复服务。然后复核平台身份并重开原Pinned，返回前再次核对状态锁。签名和关联校验前不接触端点；记录不存在时不读取凭据、不创建状态。
应用backend从该只读能力构造macOS OwnedOneBot与SocketSource；每个HTTP/WS连接继续核对实际服务端socket后才发送认证，WS重连不降级为无归属连接。恢复本身不确认账号、修改ready、发现新服务或授予进程树停止权限。
本阶段只实现读取和应用传输接合，生产代码不创建j2记录，不接受用户提供的PID/URL登记服务。后续受控启动的生产者必须在原启动/实例权威下验证专用副本与运行成员来源、原连接归属，再发布绑定；不能把任意Pinned加上Dispatched回执视为允许接管普通QQ。控制面绑定、首次发现、安装/启动生产者及完整日志迁移仍待实现。
签名材料遗失时macos-runs索引也阻止创建替代密钥，即使references目录已缺失。

## 验收界限

原型验证启动、GUI、真实初始化、身份一致及 10 个读取接口。macOS Rust CLI 尚未接合。
私聊历史、写入、媒体、事件/恢复、多实例、更新及长期稳定性分别登记 TASK-006，不能用本原型结果勾选。
不支持 Intel/Rosetta；最低 macOS 版本未由单机实验确定。OCR/WebUI 终端的上游限制见 FACT-005。
