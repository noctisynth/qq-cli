# CLI 核心模块离线验收

文档编号：CHECK-013。状态：核心模块定向验收，完整功能未完成。更新：2026-10-18。
关联：PLAN-001、SPEC-004、TASK-002、RFC-0002、RFC-0003、RFC-0004。

## 安装来源编排增量（2026-10-18）

完整调度绑定增量完成后，当前完整196项离线回归、fmt和全目标clippy通过（包含三个后续增量的新增测试）；Semifold解析18个未消费changeset。此前193项为历史完整回归记录，不作为当前总量。未执行真实QQ操作。

完整调度绑定增量：dispatch模块串联锁内输入复核、已准备材料复核、原版三参数及环境白名单，版本化摘要覆盖兼容键/Plan/程序参数cwd/过滤环境/配置摘要；launch_in在原生入口前重算及比较完整Target。新增1项原创布局测试证明三参数和工作目录环境生成、忽略NODE_OPTIONS不改变摘要、允许的APPDATA变化改变摘要；真实start_intent事务内错误Target在分配Job前拒绝，无run-jobs，随后正确Target成功进入同锁原子创建。测试以rustc在自有临时目录编译原创三参数等待程序作为napimain替身，程序只写自有启动标记并等释放文件或15秒自退；父验证原始Pinned/Job成员并释放等待退出。DLL/CJS为原创非执行字节；不读取QQ账号或加载DLL/JS，artifact检查为假探针，不能充当真实产物/原版兼容或受检加载证明。runtime_store共7项、已有run_job原创子进程2项通过。公开新建、完整原版正向/文件加载绑定及登录恢复未验收。

运行材料锁内复核增量：新增verify_prepared，prepared/starting阶段只读匹配r1、登记运行/安装/端点、受保护整组秘密与独立API值、目录及配置。新增1项原创用例覆盖两阶段正向、ready阶段在凭据读取前拒绝、配置修改保留、缺API密钥不补建及安装摘要变化拒绝；无额外凭据写入、无登记改写。随后在真实start_intent的现有事务内执行材料检查及原创计数回调，Dispatched仅表示回调返回，无run-jobs或进程。runtime_store共6项、fmt与全目标clippy通过；最近全量回归仍为193项，新增定向测试不冒充全量回归。公开新建、运行后恢复、真实加载仍待完成。

锁内输入复核增量：Inputs.with_revalidated在调用者现有Transaction内重跑完整检查，比较完整兼容键、QQ实际可执行路径及安装ID/版本/摘要/目录；一致才调用后续操作。新增1项原创用例证明操作内另一事务返回STATE_BUSY，撤销证据、产物拒绝、换安装目录及QQ文件改变（即使重新提供匹配的新证据）都不调用后续操作；未新增start-requests/run-jobs。startup共5项定向测试及全目标clippy、fmt通过，最近完整回归仍为前增量193项。本次不宣称194项完整回归或原版加载验证；该接口尚未接公开新建调度，状态锁不能排除外部文件改写。

启动输入增量完整193项离线回归、fmt与全目标clippy通过；新增4项定向测试。Semifold解析15个未消费changeset，未执行version/publish。

后续启动输入增量：startup只读编排接到公开start新建检查后方；4项原创测试覆盖授权/截止先于产物与QQ访问、安装缺失/重复/摘要/完整性拒绝、QQ缺失/相对路径/超16候选/歧义拒绝，以及重复来源去重、精确Framework支持与拒绝、Shell证据不足、指纹变化失效。所有路径都未创建状态，假探针api调用会直接测试失败。正向仅证明输入检查，不证明原版启动、锁内最终文件绑定或实例生命周期。当前生产授权依据和完整兼容证据仍为空。

后续start复用增量：原创受管CLI场景增加start正向、版本冲突、--new许可拒绝和账号变化拒绝；注册表字节及启动意图数量保持不变，累计API请求22。6项受管传输测试、完整189项离线回归、fmt与全目标clippy通过；Semifold解析13个未消费changeset。不启动真实QQ，不验证新建生命周期。

本次完整189项离线测试、fmt与全目标clippy通过；Semifold解析11个未消费changeset并保持单发布单元。测试使用原创归档/临时目录，未联网下载或操作QQ。

新增acquire惰性输入模块及公开install分支，目录/许可先于凭据和来源访问。3项原创集成验证本地合成归档导入、删除源文件后缓存复用、丢失/错误长度/同长度摘要损坏拒绝，以及独立CLI对未确认许可/未知版本不创建状态、不泄漏输入路径。2项单元验证HTTPS跳转目标限制及输入打开失败不重试；无真实下载或QQ操作。网络传输只经过静态规则检查，真实下载、网络故障全矩阵与完整安装任务仍未完成。

## 状态版本门禁增量（2026-10-18）

status在记住或确认账号前复用Session固定版本校验。原创Job服务同时改变合成账号和版本，CLI返回VERSION_MISMATCH且登记账号保持原值；恢复版本后业务读取仍返回ACCOUNT_CHANGED。6项受管传输测试及完整184项离线回归通过，fmt与全目标clippy通过；该用例业务请求调整为16，记录拒绝仍无API回退。不读取真实QQ，完整启动/登录验收仍未完成。

## 严格受管CLI与夹具迁移增量（2026-10-18）

新增1项独立CLI子进程测试：通过既有s1/j1/t1流程原子创建原创服务、发现实际TCP服务Pinned并签名绑定，然后关闭父测试的所有Job/进程句柄。独立qq-cli进程必须恢复签名服务，status输出首次实际合成账号，contact list读取并带同一作用域；改合成账号后返回ACCOUNT_CHANGED且不返回联系人。损坏j1保持原字节并失败、暂移本次自有j1后返回RUNTIME_RECORD_REQUIRED且不重建，精确请求计数13证明两种记录拒绝都无额外API调用；归属探测累计零字节连接1次，不算业务请求。全部来源/账号/联系人均原创，不读取真实QQ。

tests/common/managed.rs提供仅测试的Rust转发进程，运行测试二进制自身的精确单项入口，最多40秒且由自有释放文件结束；不提供生产CLI开关，不造签名证明。通过正常意图/Job创建和服务绑定把原HTTP/WS假服务置于可信进程后，Peek到应用字节才向原假服务连接，零字节探测不消耗原请求断言。原脚本处理器、Bearer/请求/正文/写意图/计数、媒体原子发布、NDJSON和阻塞输出检查保留。send/reply共8种组合、file私聊/群2种、event正常/阻塞2种均迁移并通过；父本地创建句柄关闭后由子进程继承/保留的QUERY句柄维持Job。回执在停止/凭据不可用时仍离线复用。该转发只证明CLI与可信原生服务的协议组合，不证明原版QQ加载或网络行为。

transport彻底删除普通HTTP/WS回退：无可信运行记录一律失败，非规范凭据键也无例外。所有应用及NativeProbe入口已无OneBotClient/SocketSource::new调用；低层OneBotClient仅留给独立协议模块测试，不构成公开接入模式。SPEC-004、PLAN-001与README同步严格契约，旧临时分支被本增量替代。完整启动/登录恢复、真实协议与事件全部故障仍未验收。

最终184项离线测试、fmt/clippy、release、116源码包文件、Semifold及文档图/引用/隐私/计数检查通过。TASK-002现54/87、配置传输发现14/18，只增加严格CLI与夹具迁移子项，不勾选完整生命周期。暂存区为空，不提交，不运行NapCat/QQ或真实会话。

## 客户端工厂接合增量（2026-10-18）

transport::Context由普通命令、event与NativeProbe共用：有j1指针先恢复签名Service，规范runtime-api标识缺指针直接RUNTIME_RECORD_REQUIRED；记录损坏、签名/安装/完整身份/端点错误不降级、不读取API token。Context保留实例快照，Client不接受换实例/run/安装/凭据键/HTTP端点，Source不接受更换登记WS端点。Client以既有Backend类型擦除封装普通或OwnedOneBot，media_bytes明确转交专用路径；事件使用同一Context产生Owned Source。无新增依赖或unsafe。旧合成登记仍临时走普通传输，不能据此勾选完整客户端选择；后续必须迁移夹具并删除/拒绝旧路径。

为保持已确认的离线幂等语义，写入口将工厂/凭据错误封装为不可用Backend，仍交原Plan指纹/ledger路径；完全匹配的既有回执在调用Backend前返回，新意图在既有preflight失败，不重发或降级。读命令直接返回工厂错误。2项新增离线测试证明缺受管记录不访问任何凭据/不创建状态、Context不能跨run或WS端点复用。既有send/reply全CLI合成流程在停止后改用缺失运行记录的规范凭据键，8种私聊/群及已接收/未知组合仍正确复用离线回执，定向通过。既有原生服务测试加入签名Context恢复及Client构造，不发应用字节；正向受管CLI/API及完整账号关联仍待验收。定向阶段尚未增加业务勾选，最终结果与计数见本节末。

本轮原生并发纠错：完整回归再次击中根退出后Job成员非空；先允许仅原PID残留的断言仍在重复运行失败，故不能断言仅为原PID清理。进一步只对自有Job所列成员查询映像名，在第13轮捕获非原PID为conhost.exe。这是本机原创进程实验观察，不推断所有系统或QQ加载器都会产生该成员。测试改为根完整身份确认退出后，在同一固定截止内等待Job名称不可重开，每轮先关闭查询句柄，成员只用于诊断，不把根退出等同整个子树退出；未放宽生产Job/进程/服务映像门禁，也不杀成员。此前“成员必空/最多原PID”的候选断言已被本证据推翻，历史5轮通过不作为长期保证。

工厂接合复核还发现通用References::open在已有签名材料但凭据丢失时会生成新密钥；虽然后续旧签名会失败，仍不符合恢复约束。补同一状态锁内的非空受保护命名空间检查，存在引用、运行/启动/保留/安装/准备/附件材料时返回SIGNING_KEY_REQUIRED，不设置密钥，不重签或删除材料；只看各目录首项，不遍历正文，链接/访问错误拒绝。新增1项内存凭据/临时目录测试：空状态允许首次建立、已有m1丢密钥不重建且原对象仍由原References核验、即使引用目录缺失但j1命名空间留存也不重建，材料字节及set计数不变。定向通过，最终完整回归结果见本节末。

最终完整183项离线回归通过；修正退出观测后5轮29项单元复核通过，随后新增丢密钥1项纳入最终回归。fmt/clippy、release、115源码包文件、Semifold与文档图/引用/隐私/计数复核通过，暂存区为空。Clippy测试模块位置调整至文件末尾，无行为改变。TASK-002现53/86、配置传输发现13/17，只勾选共享工厂接合子项；旧合成登记迁移、受管CLI正向/账号关联及真实QQ仍未完成，不提交。

## 持久服务绑定增量（2026-10-18）

run_job的j1记录增加可选Service，旧记录缺省None；非spawned不得出现Service。bind_service在既有Transaction内核对完整Target/Job、当前安装摘要/实例阶段、BoundStream与Pinned完整身份及登记HTTP端点，复用原流归属检查。先完成t1成功保留再保存服务Identity/规范端点，不写账号或ready。已有服务不换身份/端点，不重复发布j1；只查询既存t1，缺失时拒绝，不重新复制。新增retention::existing复用同一签名/绑定/状态校验路径，仅缺日志分支禁止执行/创建，不引入第二份日志解析。

reopen_service将签名实例/run/安装ID和摘要/端点与当前所选登记逐项比较，再从完整服务Identity重开Pinned和QUERY Job、复核成员；不会搜索替代PID或读取凭据/发送API。ServiceRun无Debug/Serialize，持有真实本地句柄，Drop不终止成员。原始根仍留作创建证据，不用服务身份覆盖root。该内核不检查实际加载摘要/许可/兼容见证，不冒充完整客户端工厂或账号校验。

既有2项run_job测试扩展原创根进程回环服务：父进程用实际连接发现Pinned后绑定、重复绑定j1字节不变、恢复完整身份及成员；改变安装摘要/端点拒绝；将本次自有t1指针移到同一临时目录后，重绑失败且不新建日志，恢复指针后j1仍原样。根退出后服务恢复拒绝，子服务记录收到应用字节0。未增加独立测试数量，定向2项及完整180项离线回归通过。

fmt/clippy、release及114源码包文件、Semifold、文档图/引用/隐私/任务数量检查通过；TASK-002现52/85、配置传输发现12/16。Clippy修正测试匹配分支多余unit表达式后通过，无新增unsafe或依赖。完整CLI客户端工厂、账号关联及真实QQ仍未验收；暂存区为空，不提交。

## 连接服务发现与事件Source增量（2026-10-18）

Job::identify_connected只接收已连接字面回环流及独立验证的完整映像路径。查询同一流的服务器四元组PID后先检查可信Job成员快照，再打开该PID的只读Pinned；映像必须精确相等，不以文件名、父PID或日志猜测。随后复用bind_connected的Pinned/Job及两次四元组验证，返回同一BoundStream和Pinned。快照与打开之间身份变化仍由后续完整归属检查拒绝；不能视为任意端口的长期授权或已登录证据。系统表其他行不保留，不全局打开/枚举QQ进程，无新增unsafe或依赖。

既有3项TCP测试扩展原创服务IPv4/IPv6发现，比较完整Identity与原流本地/远端地址；错Job/映像、相对路径/截止及实际外部服务PID拒绝，服务端累计应用字节仍为0。先行定向及最终完整回归均通过。没有增加独立测试数量，不将旧测试数量重复计算为新增。

SocketSource::owned借用Job/Pinned，在同一Source中每次连接进入Connection::connect_owned；receive/disconnect和watch编排保持共享，无公开模式/参数。每次替换先断开旧连接，失败后receive不能返回旧流。既有独立WS测试改为通过该Source接收IPv4/IPv6原创事件，再用过期重连确认旧连接已不可用、请求仍仅2次；5项独立HTTP/WS测试通过。完整CLI选择及HTTP/WS账号关联未接入；原生归属不替代业务身份校验。

最终180项离线测试、fmt/clippy、release、114源码包文件、Semifold及文档图/隐私/引用/任务数量检查通过；TASK-002现51/84、配置传输发现11/15。仅增加已验收发现子项，完整生命周期与交付保持未勾选。暂存区为空，不提交，不操作真实QQ。

## 原生调度日志增量（2026-10-18）

run_job在现有启动意图dispatch提供的Transaction中工作，无嵌套锁。先核对相同请求/完整Target的s1 pending、starting实例及安装摘要，再创建Job并将随机Job ID写入j1签名pending，之后才调用现有原子spawn。已有任何本轮日志均拒绝再次创建；创建失败或最终回执写失败为START_OUTCOME_UNKNOWN，不重放。成功后记录原始Pinned Identity和spawned，不更新账号或ready。该模块不验证许可、配置内容、兼容见证或输入摘要与实际文件的对应，调用方仍须先完成所有启动门禁；不得直接暴露成绕过门禁的CLI路径。

reopen_root仅从签名记录重开完整根Identity、QUERY Job并核对成员，不枚举同名进程或搜索替代PID；加载器已退出时拒绝，后代恢复另行接合。Running无Debug/Serialize，持有真实Job与Pinned，Drop只关闭本地句柄，不终止。j1只供内部引用，不公开schema/资源读取。Spawned新增消费原Pinned的方法，不按PID重新观测原创建结果，无新增unsafe或依赖。

2项定向测试通过：原创测试程序作为根，从签名日志验证自身Job成员，父进程比对原始Identity、拒绝重复、关闭本地Job后依靠子进程继承句柄恢复，错签名保留原文件，退出后拒绝；原创非可执行文件导致创建失败，日志保留Job/unknown、pending同样拒绝再创建，错Target不创建日志、损坏日志保留。所有子进程均有15秒上限及自有释放文件，不操作QQ。完整180项离线回归、最终27项原生/单元复核、fmt/clippy、release、114源码包文件检查通过；TASK-002现50/83、运行环境14/23。完整CLI启动、许可/兼容/配置门禁、后代服务绑定及真实QQ仍未验收，不暂存、不提交。

并发回归纠错：初次完整回归在既有原子创建测试的“进程退出后名称立即不可重开”断言失败。先怀疑普通Command夹具在并发时继承临时句柄，已将该夹具替换为独立保护Job、显式继承列表及15秒/释放文件，不再用管道阻塞或kill；测试专用Assign仍仅原始创建句柄，不进入产品。替换后同一断言仍偶发失败，不能把最初猜测当作原因。随后观察证明进程已退出且Job成员为空时名称仍可能短暂存在；改成固定截止内确认消失，每次先关闭查询再等待，避免观察者保留名称。5轮27项并发单元测试通过，其中1轮经历1次10ms间隔，其余0次；证据只支持有界观测，不能保证所有Windows版本立即清理或真实QQ行为。生产实现未增加重试、杀进程或重建Job。

## 一次性保留日志增量（2026-10-18）

retention在调用方现有Transaction中编排，不另取状态锁。按instance/run哈希确定日志路径，t1签名记录绑定完整JobId和Pinned Identity；原子发布pending先于调用原生保留。历史结果仅在绑定完全相同时返回，pending转unknown且不重做；已停止的登记可读取历史结果，但不代表当前远端句柄或进程有效。新操作限登记run及starting/login_required/ready阶段，过期/错目标/签名错误不降级。

3项日志定向测试已通过：回调观察到先行pending、停止后历史复用/换Job拒绝/错签名保留原文件；最终写入被原创临时文件阻断后返回RETENTION_OUTCOME_UNKNOWN，恢复日志后pending转unknown且重复不执行；无效阶段/截止不派发，操作失败未知且不重试。合成回执仅用于日志状态测试，不作为内核句柄使用。既有原创后代原生正反对照已改用公开retain_once路径并比较缓存回执，完整178项离线测试通过；不得据此宣称真实QQ兼容或完整生命周期完成。

本增量TASK-002为49/82，运行环境13/22；fmt/clippy、release、113源码包文件白名单、Semifold配置/状态检查通过，暂存区为空。Clippy仅出现环境增量缓存硬链接回退复制提示，无代码诊断；不改变全局构建设置。未操作真实QQ，不提交。

## 受检WS与握手预算增量（2026-10-18）

Connection::connect_owned在唯一共享connect_with路径中取得TcpStream后，通过Job::bind_connected确认原socket，再交既有tungstenite握手；不新建连接，Upgrade/Bearer写入前必须成功。握手返回后复核同一Pinned与Job成员；失败时局部Connection按Drop关闭底层socket。普通connect仍走同一实现的直接验证回调，共用路径、凭据/URL校验、敏感头、握手错误脱敏、帧/控制/截止/关闭行为，不维护第二份WS协议。

SPEC-004和PLAN-001同步握手阶段累计读入32KiB，成功后才提高到全流64MiB（不清零已收字节）；写64KiB、单帧/完整消息4MiB和10000帧不变。接收包装器在每次读取前按当前阶段余量缩小buffer，不因握手依赖内部扩容接受巨大响应头。所有I/O使用同一总截止；未接入常驻监听/自动写入或全局代理。该预算也用于普通路径，公开方案及README明确内核验收不等于完整托管启动。

新增2项Windows原创独立服务测试：Job原子启动的自有服务分别在IPv4/IPv6接收Upgrade，检查合成Bearer、Upgrade头、无cookie/proxy认证后返回有效101和原创文本帧，客户端只在原连接消费；错Job、过期、非法凭据及实际服务进程不符均无Upgrade，目标端读到0字节，端点计数证明零请求。复用原HTTP服务/原生能力夹具，测试文件按职责迁为owned_transports，原3项HTTP全部通过。新增1项普通WS超大响应头拒绝，既有认证/重定向/文本/ping/close/二进制/超帧/截止测试均重跑通过；未代替HTTP/WS账号关联、事件补读或真实协议。

TASK-002现48/81，配置/传输/发现10/14；新增勾选只指受检WS通道。完整175项离线测试、fmt/clippy、release、112源码包文件白名单、文档图/链接/隐私/计数、Git diff及Semifold复核通过，无新增依赖或unsafe；暂存区为空，不提交。托管客户端/watch选择、完整跨通道身份、生命周期和真实QQ兼容仍待完成。

## 受检HTTP Backend增量（2026-10-18）

owned_http无unsafe，实现既有Backend接口，借用已认证的Job/Pinned；不构成启动许可。普通响应使用共享client::parse_envelope，媒体使用共享media_transport::from_slice，原reqwest通道亦调用同一函数，不维护两份retcode/async/媒体元数据规则。同步构造只验证配置/凭据/字面回环端点与创建本地current-thread运行时，无网络。已有Tokio上下文明确返回HTTP_RUNTIME_CONTEXT，避免嵌套阻塞panic；公开托管客户端工厂仍待接入。

每次请求一个TcpStream，归属核对后原流移入TokioIo，Hyper低层HTTP/1握手/发送器只使用该连接，无连接池/代理/跳转/新连重试。固定Action路径与Host/JSON/identity/Connection-close；Bearer HeaderValue标Sensitive且只在核对后插入请求，不回显底层库错误或正文。受限writer序列化请求最多4MiB；响应普通Config预算1KiB至4MiB、媒体MAX_BASE64_BYTES加同一元数据预算。Hyper处理Content-Length/分块/EOF/截断及冲突分帧，头/尾部32KiB、64头、缓冲32KiB，帧累积前核对字节数；拒绝非identity内容编码、非chunked传输编码。401/403、其他非2xx、retcode失败/async、畸形JSON分别策划错误，保留无损整数，不传播原message/url/path。

绝对deadline先取命令截止与单请求配置最小值，覆盖编码、连接/绑定、异步握手/发送/读取及返回前身份复核；timeout_at使用同一绝对Instant，握手/发送前再检查，不把进入运行时的延迟补到预算。连接任务有Drop-abort护卫，正常/错误时abort并等待取消，超时/取消也中止，不在后台等keepalive；未承诺抢占同步内核调用或解析CPU。媒体字节仍单独有界，未令普通消息接受巨大响应。

3项Windows独立原创服务集成：通过Job原子启动自有服务、IPv4/IPv6分别验证固定长度/分块及尾部、64位ID、普通预算拒绝较大媒体而专用通道成功；认证/重定向/retcode/async/JSON、声明/分块超限、gzip声明、超长头、冲突Content-Length、截断及单次写请求异常均不重试，陷阱端口无人连接，错误序列化无合成token/message。500ms截止中止慢响应，服务标记证明请求已进入；请求计数精确核对14次且无重发。错Job、错实际服务进程、过期、4MiB请求超限、非法凭据和嵌套运行时不产生额外认证请求，非目标服务读到0字节。

最初测试假服务从非阻塞listener接受socket后未恢复阻塞，尚未收到HTTP即按WouldBlock关闭，导致严格TCP归属拒绝。曾尝试有界观测但未解决，确认为夹具行为后撤掉额外轮询，修正假服务的阻塞/超时模式；不放宽实际归属检查或重连。最终测试在原严格核对下通过；握手使用原子夹具文件，无时间猜测等候。服务及后台线程只属本测试进程，25秒退出界限和仅自身释放文件，不触及QQ。

直接依赖沿用原图版本：hyper1.12.0、hyper-util0.1.21、http-body-util0.1.5、tokio1.53.3，包元数据均MIT；Cargo.lock固定版本/校验，未引入新版本或复制源码，不代表最终依赖通知/上游NapCat许可齐备。[Hyper1.12.0连接Builder](https://docs.rs/hyper/1.12.0/hyper/client/conn/http1/struct.Builder.html)说明连接级IO接口及头部预算；库源码/清单本地固定版本观察，检索2026-10-18。未自写HTTP分帧器，不据依赖默认值假设安全预算。

TASK-002现47/80，配置/传输/发现9/13；勾选只指独立受检HTTP Backend，App/托管完整选择、WS及真实QQ仍待完成。完整172项离线测试通过，之后仅新增同一测试内的运行时上下文拒绝断言，3项受检HTTP再回归通过；fmt/clippy、release、112源码包文件白名单、文档图/链接/隐私/计数、Git diff和Semifold复核通过，暂存区为空。不调用真实QQ API，不暂存、不提交。

## 已连接TCP归属增量（2026-10-18）

native_job/tcp在现有唯一原生边界内使用固定windows 0.62.2的IpHelper/WinSock绑定，不新增crate版本。Job::bind_connected消费调用方已经连接的TcpStream，限制两端字面回环/非零端口/同族/非同端点、IPv6 scope/flow均零；调用前不得有协议写入。服务器行按local=流peer、remote=流local的精确IP/端口四元组、ESTABLISHED状态匹配，PID须等于可信Pinned。前后核对Job成员/完整进程身份，两次TCP快照及流地址不变后返回无Debug/Serialize的BoundStream；HTTP/WS必须使用其into_stream返回的同一个socket，不能另行连接。

GetExtendedTcpTable单次表缓冲上限2MiB/8192行，最多3次大小增长，16字节对齐的已初始化缓冲区、SDK字段偏移及完整行长度检查；IPv4/IPv6布局均仅整数与字节数组，逐行read_unaligned。地址按原生内存字节、端口按低16位网络字节序解释，忽略端口未用高位。缺失/重复匹配、零PID、非established或不可信PID均拒绝。系统接口返回非目标行是不可避免的临时元数据，解析后丢弃，不持久/输出其他进程/地址，不调用其应用接口或读取聊天；错误只输出策划代码，不回显系统表/路径。该快照不保证恶意已信任进程随后不转移socket，不等同已验证加载内容。

3项原创测试：合成SDK数字行验证方向/网络字节序/端口高位、短表、过量计数、重复、listener、零PID、自连接和非回环拒绝；当前测试进程自有IPv4/IPv6真实TCP对照均匹配当前PID且服务端无可读数据，过期拒绝；独立原创服务进程由本工具Job原子启动，在IPv4/IPv6发布自身端点，正确连接返回同一socket与服务进程Identity，跨Job、过期及连接到父测试进程服务而仍声称子服务PID均拒绝。服务端确认总应用字节数为0；句柄/握手文件仅属随机临时测试目录，服务有15秒有界退出。未发送OneBot请求或凭据、未触发QQ应用操作。

依据[GetExtendedTcpTable](https://learn.microsoft.com/en-us/windows/win32/api/iphlpapi/nf-iphlpapi-getextendedtcptable)、[MIB_TCPROW_OWNER_PID](https://learn.microsoft.com/en-us/windows/win32/api/tcpmib/ns-tcpmib-mib_tcprow_owner_pid)、[MIB_TCP6ROW_OWNER_PID](https://learn.microsoft.com/en-us/windows/win32/api/tcpmib/ns-tcpmib-mib_tcp6row_owner_pid)；检索2026-10-18，可变官方来源，不作NapCat固定基线。当前只验收连接观测内核，未测试所有缓冲增长/接口错误/端口重用和恶意socket转移矩阵，也未把现有reqwest请求或WS握手接入该能力。

TASK-002现46/79，配置/传输/发现8/12；只单列连接归属内核，不勾选完整HTTP/WS身份关联、托管入口或真实QQ回归。完整169项离线测试、fmt/clippy、release及110源码包文件白名单通过；文档图/链接/隐私/计数、Git diff与Semifold复核通过，暂存区为空。最后仅补充调用前不得有协议写入的API注释，未改控制流。不调用真实QQ API，不提交。

## Job内后代句柄保留增量（2026-10-18）

native_job/retain保持原生边界隔离：目标先由Pinned完整身份及Job成员快照核对，再仅以DUP_HANDLE/QUERY_LIMITED_INFORMATION/SYNCHRONIZE打开，借共享pin所有权转移助手复核PID/原始创建值/映像。使用同一目标句柄在复制前后IsProcessInJob复核，两个Pinned均再次确认存活。DuplicateHandle只复制本Job的QUERY权限、noninheritable句柄，不读写目标内存/执行代码/创建线程/终止，不接管外部进程。复制后失败保守返回RETENTION_OUTCOME_UNKNOWN且不标retrySafe；调用方仍须先持久一次性意图，不能据报错自动复制多次。

Retained仅为私有序列化回执，无Debug/公开输出适配，包含Job ID、完整Identity和目标上下文的句柄数值。该数值永不在本地CloseHandle，不按陈旧数值远程回收；目标自己关闭或退出时由OS释放。它不是“目标未来永不关闭此句柄”的保证，也不代替签名日志或实际加载/监听绑定。共享pin助手只移交成功返回的原生进程句柄，避免按PID重新猜测与重复关闭；普通Pinned新增只供原生边界借用的句柄访问，不增加内存/终止接口。

2项原创Windows测试：非成员（仅当前测试进程）和过期在复制前拒绝。父子正反对照均仅运行当前测试可执行文件；root原子入A Job，后代原子入nested B Job、明确继承列表排除A句柄，同时仍是A成员。后代先释放自己按名打开的A查询句柄，再以原子私有握手文件报告Identity。未保留分支root退出、父侧Job关闭后，后代仍活着但A名称不可重开；保留分支后代验证QUERY可用、ASSIGN拒绝和INHERIT标志关闭，再证明root退出/父句柄关闭后仍可重开A且核对相同Pinned。后代最终退出后A名称不可重开。握手/回执原子发布，两个测试程序有15秒有界退出和仅本临时目录的释放标记清理，无QQ进程/账号/API操作。

最初无保留分支用普通测试启动器时A句柄被意外继承，导致名称仍可打开；该失败不能证明DuplicateHandle保留生效。改用明确HANDLE_LIST与nested Job构造后，两分支差异才成立，不把普通启动器行为推广成真实原版加载器结论。未实测真实QQ安全检查、跨位数、句柄被目标提前关闭、全部访问/复制/落盘故障或完整崩溃矩阵。一次性持久保留编排和公开start/stop仍未接入；不据这2项测试勾选整体生命周期。

依据[Microsoft DuplicateHandle](https://learn.microsoft.com/en-us/windows/win32/api/handleapi/nf-handleapi-duplicatehandle)和[IsProcessInJob](https://learn.microsoft.com/en-us/windows/win32/api/jobapi/nf-jobapi-isprocessinjob)；检索2026-10-18，可变官方来源，不作NapCat固定复现基线。SDK明确远端值仅在目标上下文有效、目标需DUP_HANDLE权限、inherit=false关闭传播；当前测试验证本机原创后代，不推定账号安全或真实QQ兼容。

TASK-002现45/78，运行环境12/21；新增勾选仅后代保留内核，持久编排/实际绑定和真实兼容仍待验收。完整166项离线测试、fmt/clippy、release及109源码包文件白名单通过；文档图/链接/隐私/计数、Git diff和Semifold复核通过，暂存区为空。不运行真实QQ，不提交。

## 原子根进程创建增量（2026-10-18）

native_job/spawn子模块集中原生启动，仍在唯一允许unsafe的边界内。CreateProcessW携带JOB_LIST与HANDLE_LIST，在创建阶段入Job且仅传一枚QUERY权限的Job副本及一枚NUL读写设备副本；父Job及原生进程/线程句柄不被继承。QUERY重开的Job不能调用spawn。无需先创建/挂起再assign/resume，不增加挂起孤儿窗口，不经shell。显式绝对本地应用名/目录，拒绝UNC/设备/相对路径与链接（内部标准流仅固定NUL设备）；只验证形状，调用方仍须全部许可、所有权、摘要和精确兼容门禁，并先持久Job ID和启动意图。

属性缓冲区16字节对齐、初始化请求上限64KiB；属性数组/句柄存活至DeleteProcThreadAttributeList完成，环境块/可写命令行/路径缓冲区存活至CreateProcessW返回。输入无Debug/Serialize。标准CRT引用/反斜杠编码，保留UTF16，不透传原始命令行；最多128参数，显式环境最多128项/每键128ASCII字符，大小写重复键拒绝、排序大写键、双NUL结尾，空块不继承父环境。命令/环境硬限UTF16 32767单元；值中NUL拒绝。显式NUL标准流及CREATE_NO_WINDOW不向父控制台/管道转发上游日志；不据此承诺原版GUI/焦点行为。

成功返回的原始hProcess直接移交winsafe RAII并构建Pinned，不按数字PID重开；hThread只关闭，不挂起/恢复。创建成功后身份/成员/截止复核失败是START_OUTCOME_UNKNOWN，禁止自动再创建；明确创建失败为START_FAILED且不标retrySafe。Job副本只读，root存活可保持名称，root结束时由OS关闭。原版加载器是否把该句柄保留到实际QQ后代未验证，不能据此宣布全部跨CLI生命周期可用；该内核尚未接公开start入口。

3项原创测试覆盖编码/配额/空环境与重复/非法键、NUL/超长/UNC拒绝；过期、QUERY-only、无效参数及原创非可执行文件原生失败且Job无成员；只启动当前测试程序，子进程自身检查成员、参数含空值/空格/引号/末尾反斜杠/中文、显式环境不含PATH、Unicode工作目录、三标准流为同一NUL句柄、输入立即EOF及输出写入成功。父侧关闭所有Job句柄后凭root继承的只读副本仍可重开并核对Pinned；发送本地夹具释放标记后root退出，Job名称不可重开。测试child有15秒有界退出及释放标记清理，无真实QQ操作。首次完整回归因“运行后句柄仍应null”断言失败，布尔诊断确认工作目录正确、三句柄均非null且非INVALID_HANDLE_VALUE；未推定未证实的运行时原因，改成显式NUL并验证实际行为，没有放宽为任意有效父管道。此后3项定向测试通过。

官方依据[UpdateProcThreadAttribute](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-updateprocthreadattribute)、[CreateProcessW](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-createprocessw)；检索2026-10-18，可变来源，未当作NapCat固定基线。SDK规定属性值存活至属性列表删除、JOB_LIST可用于Windows10起、HANDLE_LIST需显式允许继承；本地测试仅证明当前Windows自有程序，不证明旧系统和真实原版兼容。

并发库回归还暴露退出观测竞态：映像查询失败时进程尚未返回明确退出状态，原测试把首次无法观测当退出而断言失败。process模块现于元数据失败后零等待复核同一对象，只在signaled时报告PROCESS_EXITED；否则仍是PROCESS_UNVERIFIABLE或截止，不把访问错误推定为退出。测试有界等待确证退出。17项库测试及2项原生进程测试重跑通过；不宣称完整系统竞态矩阵。

TASK-002现44/77，运行环境11/20；单列原子root创建内核，不勾选整体start、实际QQ绑定、后代保管或真实焦点。完整164项离线测试通过，合成NUL输出探针未出现在父测试日志；fmt/clippy、release、108源码包文件白名单、文档图/链接/隐私/任务计数、Git diff及Semifold复核通过。暂存区为空，不改NapCat，不运行真实QQ，不提交。

## 命名Job查询增量（2026-10-18）

native_job使用固定Microsoft windows 0.62.2绑定，Cargo.lock锁定依赖；包许可元数据为MIT OR Apache-2.0，未复制上游源码。原winsafe 0.0.29无Job接口；另查win32job 2.0.3公开接口仅匿名创建，无按名重开，故不为此引入该包。Cargo unsafe_code从forbid改为deny，只有lib中显式隔离的native_job允许unsafe；每个调用/指针读取均有内存、所有权或生命周期安全注释，不扩散到领域/HTTP模块。安全边界与依赖通知仍属最终交付审查，不据当前包许可元数据声称NapCat已获授权。

Job创建采用256位CSPRNG随机Local名、owner/SYSTEM受保护DACL及不可继承句柄，清空线程旧错误后核对ERROR_ALREADY_EXISTS，不接管既存对象。不设置资源/kill-on-close/breakaway标志，查询发现任何limit flags则拒绝。重开只申请QUERY，严格检查名称形状但不把名称当所有权证明；调用方仍须认证私有生命周期记录。RAII关闭HANDLE与LocalAlloc描述符；不暴露终止/注入接口。成员快照最多256项、固定已初始化且对齐的缓冲区、检查实际长度/计数/范围/非零无重复PID；contains对Pinned进程前后verify，不能替代实际加载/监听证明。

2项Windows库测试只创建随机Job和当前测试可执行文件的协作子进程。创建空列表、不同Job不包含成员、当前Job成员精确匹配、既存拒绝、QUERY重开不能assign、非法/过期ID拒绝、空Job关闭后不可重开均验证。测试专用assign接口仅cfg(test)，孩子先等管道指令，明确不作为生产原子入Job实现。清理只处理测试返回的子进程句柄，无全局QQ枚举/停止/注入。

首次测试反证初始架构推断：关闭全部Job句柄后原测试子进程仍活着，但OpenJobObject不能重开原名；保持任意查询句柄时可关闭创建句柄并再次重开。因此“保存名称、CLI退出后直接按名恢复”不足以落地，RFC-0003已修正，须保留存活句柄并验证跨进程保管或受控继承，以及创建时原子入Job。当前未冻结该机制、不默认添加全局服务；不将对象存活与名称可打开等同。未实测跨用户ACL、256成员上限、所有原生错误和原版QQ子树兼容，整体生命周期未验收。

官方来源：[CreateJobObjectW](https://learn.microsoft.com/en-us/windows/win32/api/jobapi2/nf-jobapi2-createjobobjectw)、[QueryInformationJobObject](https://learn.microsoft.com/en-us/windows/win32/api/jobapi2/nf-jobapi2-queryinformationjobobject)、[Job Objects](https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects)、[SID Strings](https://learn.microsoft.com/en-us/windows/win32/secauthz/sid-strings)；检索2026-10-18，均可变来源。官方对象销毁描述不直接证明最后句柄关闭后名称仍可打开，以上结论以本机原创子进程实验为界。Rust标准库spawn_with_attributes当前公开文档仍标nightly，未升级固定工具链或绕过稳定版限制。

TASK-002为43/76，运行环境10/19，新增勾选仅Job查询内核；跨CLI句柄保管、原子spawn及真实归属仍待完成。完整161项离线测试、fmt/clippy、release及107源码包文件白名单通过；文档图/链接/隐私/计数、Git diff和Semifold复核通过，暂存区为空。第一次预期可重开的测试因真实行为不符而失败，随后依据原生证据修正机制与测试，未改成声称完整生命周期已验收。

## 持句柄进程观测与启动run认领增量（2026-10-18）

process模块只对调用方给定PID打开不可继承的QUERY_LIMITED_INFORMATION|SYNCHRONIZE句柄。零等待检查前后读取同一内核对象的PID、原始64位FILETIME创建值和Win32映像路径；不使用GetExitCodeProcess的STILL_ACTIVE值猜测存活。Pinned保留原句柄，verify不重新打开PID；reopen核对完整Identity，创建值或映像不一致拒绝。Identity不实现Debug，路径只供内部私有记录，未加入公共输出。无枚举、调试特权、内存读取、终止或注入接口；不能据观测证明加载内容/父子/监听归属。固定winsafe路径缓冲区有限，错误拒绝，无脚本回退。

2项原创Windows测试：只启动当前测试可执行文件，持管道等待父测试指令；原句柄观测、重新打开、创建值和映像不符、非零原始创建时间，以及子进程退出259仍检测PROCESS_EXITED。零PID、过期和非法私有Identity在调用原生观测前拒绝。清理只对当前测试取得的子进程句柄操作，不枚举或操作真实QQ。未实测系统PID回收，使用记录创建值不匹配验证拒绝路径；未证明系统拒绝访问/长路径全矩阵。

依据Microsoft官方[OpenProcess](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-openprocess)、[GetProcessTimes](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-getprocesstimes)、[QueryFullProcessImageNameW](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-queryfullprocessimagenamew)及[WaitForSingleObject](https://learn.microsoft.com/en-us/windows/win32/api/synchapi/nf-synchapi-waitforsingleobject)；检索2026-10-18，均可变来源，不作NapCat固定复现基线。实现使用既有winsafe 0.0.29安全封装，项目不新增unsafe或依赖。

审查修复start_intent持久顺序：先保存按实例/run确定的不可变s1认领，再保存请求索引及starting。若在后两次写入前崩溃，旧prepared不会允许换键调度；同键认领只能恢复unknown。已有请求也须核对认领；缺失/损坏认领拒绝。新增第7项启动意图测试合成“认领存在、索引未写、登记prepared”布局，验证换键拒绝、同键unknown及索引恢复，不调用调度。此前6项（包括独立进程/主动退出和回执落盘失败）均重新通过。完整落盘/电源中断矩阵、真实启动、父子和监听归属仍未完成。

TASK-002现为42/75，运行环境9/18；进程观测只单列内核验收，不勾选实际绑定或完整status/doctor。完整159项离线测试单编译任务通过；上述9项定向测试均重跑通过。此后仅将冲突错误改为“no new dispatch”避免误读，未修改控制流，7项启动意图再复核通过。fmt/clippy、release与106源码包文件白名单通过；文档图/隐私/任务计数、Semifold与diff复核见本次工程检查。不以离线证据冒充真实协议/窗口验收。

## 启动意图上一增量（2026-10-18）

该阶段累计156项离线测试通过：此前150项加start_intent的6项；先完整155项单编译任务回归通过，再新增独立进程/主动退出测试并定向运行全部6项。fmt/clippy通过，release构建通过，源码包104文件白名单通过。默认完整脚本首次在并行链接时遇到MSVC LNK1104无法打开msvcrt.lib；不改全局环境，单编译任务测试通过。后续仅补充测试及模块注释，未改业务实现。

新增start_intent模块：调用方提供经过门禁的启动输入指纹，内核在状态锁内核对prepared、run、安装摘要及无账号/进程字段，先持久s1签名记录再登记starting，最终提供当前持锁调用的一次调度闭包。重复键对比完整目标，不同输入拒绝；遗留pending转unknown，不重新提供调度机会。闭包错误按未知处理，最终回执无法落盘返回START_OUTCOME_UNKNOWN及请求键，retrySafe=false。调度成功只记dispatched，绝不据此写入ready、账号、PID或进程创建时间。读记录不创建状态、不探测端点或凭据。

6项原创测试覆盖：落盘顺序/调度后状态、同键复用及不同键/输入冲突；丢弃意图与闭包错误；过期/非法阶段/损坏指针不覆盖；双线程竞争只一次；锁截止及最终回执失败后恢复unknown；两个独立测试进程竞争只一次，以及测试子进程在假调度记录后主动exit(23)、再次调用仍unknown不重放。子进程只运行当前测试可执行文件；超时清理仅针对本测试返回的子进程句柄。没有启动QQ、原生加载器或NapCat，没有使用真实账号/消息。

该阶段TASK-002为41/74，运行环境8/17；将启动意图细项单列完成，不勾选整个start任务。实际安装/许可/材料/兼容门禁、真实进程和监听归属、启动成功/失败恢复及完整跨进程故障矩阵仍待完成。当前核心尚未接入公开start入口，不作为真实启动验收，也不因此允许读取计划端点或终止用户进程。

## 已实现与验证

- library/binary 分离；CLI、注册表、领域标识、错误/输出、配置、状态、系统凭据、传输、会话门禁、集合查询和引用分别封装。
- 34个目标命令解析/帮助已迁移，离线 schema list/get 从 Clap 生成，含基础类型/约束；完整领域响应 schema 仍待补全。
- 紧凑 JSON、语义参数先校验、互斥文本源、显式创建请求键、无回显错误、子秒时间边界处理。
- 配置相对目录以配置位置解析，缺省读取不初始化；原子状态提交、文件锁截止、实例选择不猜最近活动或就绪实例。
- Windows 系统凭据适配、私有目录权限和带 HMAC 的短引用；Windows凭据适配及CLI签名卸载已有随机临时命名空间原生验证，真实QQ运行凭据和原生进程生命周期未验收。
- HTTP 字面回环/显式端口、Bearer、无代理/跳转/重试、请求截止、4MiB默认上限、整数无损和错误脱敏。
- 部分好友/群/最近会话适配，领域归一化不透传私有字段，最近会话不扩展搜索，也不输出上游最后消息正文。
- 身份/版本/在线/策略门禁，前后校验、切号结果不返回；集合续传恢复查询、作用域与快照校验，满页成功，引用只持久化定位元数据。

- 消息引用绑定账号/实例/run及后端ID、时间、序列、发送者，拒绝短ID复用；归一化移除URL与未知段原始数据，保留不支持标志。
- get --expand 单次取消息，转发展开有深度/节点/字节上限；媒体只输出定位元数据，不自动下载。
- 私聊/群历史与范围搜索合成内核、newest_first、锚点重叠去重、待返回定位信息续传及查询恢复；拒绝/停滞/预算为部分结果。
- 写意图在副作用前原子落盘 pending；崩溃残留转 unknown 不重领，账号/操作/摘要冲突拒绝，最终落盘失败返回 unknown。
- request get 离线读取不需实例在线；显式配置不依赖默认用户目录。status 可确认新的实际账号，禁止覆盖并发身份变化。

- 文本/文本文件/显式管道输入及群提及适配，UTF-8/64KiB和禁止隐式CQ解析；管道输入有总截止，交互终端不提示等待。
- send/reply 共享内核在落盘写意图后复核身份和回复目标；已知接受结果无后端调用复用，未知/拒绝键不再执行。
- 两个独立测试进程共享键只有一次假写，另一测试进程在假写后主动退出，恢复进程将其残留标为unknown且不重发。
- 本组为CLI输入与发送内核合成证据，真实凭据/受管实例全流程、附件和全部回复/权限组合仍未验收。

- 独立export共享历史Page类型；message行和末尾签名checkpoint同次原子文件替换，默认不覆盖，resume验证正文摘要/行数/路径与作用域。
- 导出部分结果保存可续传检查点，恢复不重复；stdout仅摘要。临时输出限制当前用户ACL，累计64MiB，目录锁有截止且不覆盖异源锁标记。
- Windows锁测试发现另一句柄持锁时不能预读标记，改为原子发布完整锁标记后先取锁再读取；故障已修复，未削弱检查。
- 导出内核合成验证覆盖续查、部分结果恢复、修改/跨状态/重启/查询冲突/权限及锁截止；实际QQ、进程硬终止和外部并发编辑的全面验证仍待完成。

- skill list/get/export 在配置读取前分派；六个任务共享产品规则、命令引用与按需指引，导出8个自包含文件，不修改宿主配置或既有目录。
- 源Skill经skill-creator静态校验通过；子进程在缺失配置和默认目录环境下仍能发现/读取/导出，导出引用图完整且命令均能由注册表解析。
- Skill导出目录重命名在受限沙箱内失败，正常权限下通过；没有绕过覆盖保护。尚未进行Codex/OpenCode真实闭环验收。

- 固定v4.18.38版本目录与DATA-001的release/提交/整包及7个必要成员摘要交叉核对，适用授权字段仍为空，未接入真实安装执行。
- 原始合成ZIP先复制私有快照并校验长度/摘要，再受成员数/深度/压缩与展开大小/截止限制解包；拒绝穿越、Windows别名、大小写冲突、链接、CRC及必要成员不符。
- 校验失败和对象释放清理本次自有临时文件/目录；未用真实NapCat包执行CLI解包、没有复制上游实现进入工程。
- 本机缓存zip4.0.0/8.6.0在本地注册索引标为yanked；改锁zip9.0.4（MIT，MSRV1.88），仅获取Cargo锁定依赖，解压功能只开启deflate。未声称完整依赖许可/安全审计完成。

- 安装登记以签名所有权清单绑定确定ID、来源/适用依据、目录和文件摘要；发布后登记前失败可复用原清单校验通过的自有产物。
- 卸载拒绝已停止实例引用及目录/签名/未知文件/修改不符，持久移除日志并只删清单成员；Windows文件占用、部分删除及物理移除后注册提交失败均可重试。
- 实际执行Windows系统凭据写/读/删和独立CLI卸载测试，但只使用随机临时命名空间、原始合成包及合成签名密钥，已删除测试凭据，不枚举/读取既有应用凭据。
- 凭据服务名改用已解析状态位置，验证相对/绝对、大小写及创建前后稳定；i1所有权引用单独上限4MiB，其他引用仍64KiB，680个合成文件的大清单可验签/卸载。
- 新请求先选择实例再初始化签名状态，零候选不创建目录/凭据；续传与已有消息/导出引用只读取既有签名密钥，卸载不依赖QQ在线。非法安装ID在配置读取前拒绝。
- ZIP隐式父目录也计入4096节点预算并拒绝大小写拼写冲突；固定原包只复核SHA-256和目录元数据，806节点/无冲突，没有执行CLI解包。

## 检查结果

150项测试通过：私有准备5、运行配置3、媒体来源3、截止贯穿2、媒体CLI假会话1、发送CLI假会话2、附件清理3、附件快照4、事件Schema1、流输出3、事件CLI假会话1、订阅循环5、WS假服务4、事件归一化6、启动构造3、QQ元数据/原生平台6、分层诊断4、library12（含TLS4/原生目录发布1）、归档/目录4、CLI子进程10、集合/引用6、输入2、Windows凭据/CLI卸载2、安装内核8、消息8、媒体落盘5、历史/导出10、写意图3、发送10、Skill2、状态5、真实回环假服务7。
包含34个叶子帮助、断流/非JSON/业务拒绝/异步应答/鉴权/超限、代理环境、禁止自动重试、锁与原子状态、身份切换、过滤范围和引用篡改。
cargo fmt、clippy --all-targets --offline --locked -D warnings、cargo test --offline --locked 和release构建通过。
源码包白名单检查独立执行；本地编译缓存硬链接回退提示不影响编译结果。
严格私有ACL及回环测试在沙箱内初次失败，正常权限下通过；没有削弱权限或跳过测试。
测试使用临时私有目录/合成数据/内存后端/回环假服务及随机临时命名空间的Windows原生凭据、本机系统版本/架构，不启动或调用真实QQ；旧挂起的合成测试进程已终止并补充监听截止。

## 未完成与下一步

受管实例准备/启动/真实兼容证据、历史全部异常恢复与导出全路径回归、受管实例下send/reply完整真实协议、媒体/事件恢复及全部领域schema仍未完成；既有内核和CLI假会话进度见后续逐项证据。
消息get和转发已有基础合成验证；嵌套资源ID展开及共享循环/深度/请求/响应字节限制已有合成证据；全部预算/异常及真实协议仍需补齐。已有跨进程假写验证，不替代真实协议发送验收。
doctor已接入分层内核（CHECK-014），原生绑定/能力全路径、联网/完整install与实例生命周期尚未完成；uninstall已有合成包/原生凭据支持的CLI验证。实例清单当前为已登记状态，不宣称进程存活或身份已复核。
不能以175项离线测试替代完整协议、真实会话或两个Agent宿主验收，当前不作为完整产品交付。
TASK-002 将原HTTP/WS、媒体及输出契约聚合任务拆开，分别勾选已验收内核/条件入口，当前40/73；完整媒体来源、自动配置、其余Schema、真实协议等要求未全部满足不勾选。

## 新静态依据

固定产物与对应 NapCatPathWrapper 源码均读取 NAPCAT_WORKDIR，作为config/logs/plugins/cache的可写根。
这为共享安装/隔离运行目录提供候选依据，不证明QQ数据隔离或多实例启动已经验证。
标准库文件锁参考 https://doc.rust-lang.org/std/fs/struct.File.html ，2026-10-18可变网页；真实编译/锁测试固定Rust1.98.1。
依赖来自锁定Cargo包；未复制上游实现、下载或打包NapCat，不执行version/publish、不暂存或提交。

固定 bundle 附件静态核对补充见 FACT-001：upload_private_file/upload_group_file 已含消息发送副作用，返回file_id而非最终message_id。
后续附件状态机不可按名称拆成可重试的纯上传步骤；目前已接入附件发送内核，完整CLI与真实协议仍待验收，也未新增真实实验。
媒体/消息契约补充后的完整脚本130项、90源码包文件通过；截止贯穿补充后完整脚本132项、92文件通过，随后doctor继承截止的定向验证另见末尾。暂存区为空。
QQ元数据、精确兼容键、证据分级与doctor单独见 CHECK-014；新增仅合成应用布局/假探针及系统版本/架构读取，未读真实QQ安装或会话。

群提及预检补充：合成成员列表覆盖成功、缺失成员、异常成员字段和查询期间账号变更；dry-run 不写意图，失败零发送。成功回执再次调用不查询成员也不重发。无提及文本不增加成员查询。检查使用 get_group_member_list 的 no_cache=true；成员关系仍可能在检查后变化，不承诺发送时必然仍在群。

嵌套转发补充：顶层 get_msg 仅一次；get_forward_msg 结果和内联节点中的资源引用递归展开，资源ID不输出、不持久化正文。调用链检测循环，共享深度4、100次资源请求、1000节点/段、4MiB累计响应和总截止限制。合成测试覆盖二层资源、内联资源、循环、深度、请求及字节超限；节点内媒体仍无原消息定位，不伪造可下载引用。

历史读取回退补充：仅 BACKEND_TRANSPORT 自动缩小批次20→10→5→2，每次保留相同锚点，最小2用于包含锚点的接口；至多4次尝试且共享总截止和page_limit，backendPages包含失败的历史请求。认证、协议或业务拒绝不自动重试。合成测试证明恢复及续查无重复/无跳过、永久失败和预算耗尽仍返回部分结果与游标；全部真实协议和CLI假服务链路尚待验收。

媒体复制内核补充：仅接受调用方由受管实例所有权确定的可信缓存根列表，后端返回路径本身不提供读取授权。限制本地普通文件与128MiB，路径规范化后校验根边界，拒绝已知链接/重解析路径、UNC和危险名称，校验可选消息大小及复制前后长度/修改时间，逐块检查总截止、计算SHA-256，私有临时文件同步后不覆盖原子发布。3项合成测试覆盖成功、哈希、现有输出、根外/穿越、大小不符、超限、危险输出和截止；未运行真实缓存读取。未证明恶意并发替换文件/目录或硬链接的全部竞态已解决；可信根与运行实例绑定、API引用复核、URL来源限制、CLI接入及自有缓存清理仍待完成，不能以此宣称media download已可用。

启动构造补充：launch模块不创建进程，分别构造白名单子环境和固定原生入口参数。保留必要系统/用户目录，剔除继承PATH、代理、Node/Electron和NapCat覆盖；PATH仅QQ目录/System32/系统目录，TEMP/TMP位于已存在实例tmp，NAPCAT_WORKDIR固定实例目录，不改变父进程环境。大小/控制字符、大小写键冲突、SystemRoot/WINDIR不一致、缺失目录及QQ/工作目录重叠均拒绝。原版参数ABI依据固定审计napiLoader.bat：QQ.exe、napiloader.dll、正斜线nativeLoader.cjs，无shell拼接；三项合成测试未执行任何夹具。构造器只检查形状，不能证明目录所有权、产物摘要、用户启动授权或兼容；未来生命周期持锁验证后才可使用，进程实际加载与子进程继承行为仍待验收。

媒体引用复核补充：下载策略先于原消息API；根据签名f1引用复核原消息账号/实例/run/ID/时间/序列/发送者，再匹配附件类型、ID及已知大小/名称，缺失或变化返回MEDIA_REFERENCE_STALE，不调用来源解析接口。2项合成测试覆盖成功、附件ID/大小变化、禁用策略零消息读取及切号；URL/查询参数/控制字符/Windows保留名不输出为媒体名称，不生成危险文件ID的下载引用。未新增真实消息读取；此内核尚未接到media download，可信来源绑定和落盘后身份复核仍需完整集成。

事件归一化内核：Watch不连接网络；ready/消息/notice/gap/summary对象供后续NDJSON适配使用。策略和账号检查先于目标过滤，目标范围过滤先于消息引用生成；群与自发私聊定位、整数/字符串ID归一化去重、心跳计数、错误终止锁存。限制limit≤1000、单帧4MiB、10000帧、累计配置输出字节及总截止，去重集仅保留摘要不存正文；未类型化notice仅supported=false、不透传私密字段。4项合成测试验证普通/自发消息、去重、gap、不读取目标外消息、切号终止、权限、帧/输出/事件/截止预算及隐私。实际WS、背压/退出、重连复核与补读尚未实现，不将对象构造当NDJSON或实时能力验收。

正向WS传输内核：tungstenite精确0.30.0，仅handshake特性，无TLS/URL解析或代理；锁定14个新增传递依赖，均由本机Cargo缓存离线解析，未完成全依赖审计。仅ws显式端口回环IP，root或/event路径，拒绝userinfo/query/fragment和/api；认证敏感头，不跟随跳转、不自动重试。直接TCP连接无DNS；自有Read/Write封装每次I/O重算总截止，握手空闲≤10秒、接收空闲≤500ms，总接收64MiB/写64KiB，消息/帧4MiB，完整消息或控制帧最多10000个。处理Ping/Pong/Close，非文本数据拒绝，drop仅关闭本命令socket。4项真实回环假服务测试通过；接收socket在Windows显式恢复阻塞。未连接QQ；HTTP身份绑定、NDJSON、WS重连/补读/退出仍待集成。
依赖接口依据：https://docs.rs/tungstenite/0.30.0/tungstenite/protocol/struct.WebSocketConfig.html ，2026-10-18检索；client函数网页不可访问，实际接口与握手64KiB/512包限制由锁定Cargo源码及编译/假服务验证，不将未访问网页当证据。

订阅编排内核：Source/Sink分离，HTTP身份在首次连接及每次重连前后复核，持续订阅每5秒复核；账号变更停止不切换。断开显式gap，最多2次重连，退避250/500ms且25ms切片检查取消，共享总截止和去重状态；重连失败不自动无限恢复。紧凑JSON逐条编码换行/flush，输出失败立即退出读取并断开socket，结束summary含原因/错误码；截止后仅给summary 1秒输出宽限。5项合成测试验证重连/去重/切号/策略/取消/输出失败、重连上限、参数/截止前置失败。此内核不等于完整CLI：Sink必须实现阻塞I/O的OS截止，普通Write仅前后检查时间；Windows标准输出、Ctrl+C处理器及进程级退出已接入，见后续CLI证据，真实控制台信号触发未验收，检查点和历史补读也未完成。

订阅守卫复核：首次身份观察直接作为连接前检查，后续/重连守卫单次核对固定scope，不为无操作重复前后探测；合成重连场景断言身份调用数。读取返回后再次判断总截止，避免截止后处理帧或尝试输出gap。非法limit与初始过期截止均零身份探测/零连接，连续断开最多3次连接、3条gap并以reconnect_limit/退出7总结；完整事件入口仍待验收。

事件CLI及进程输出：main将event watch分派流入口，配置/实例/版本/策略检查后使用OS凭据与注册HTTP/WS，打印ready/message/summary；selfMessages未知时明确null。进程专用输出线程、容量1同步队列、每条确认，调用方每25ms检查截止/取消，阻塞或失败后不再输出/排队；CLI退出时不等待阻塞OS写线程。3项输出测试含实际子进程不读取stdout，1项Windows全CLI测试覆盖正常3行及1MiB合成事件下stdout不读取、退出6；使用随机临时凭据命名空间和临时loopback HTTP/WS，清理测试凭据。没有访问真实QQ。ctrlc精确3.5.2（MIT/Apache-2.0、MSRV1.69）及5个新增平台依赖由Cargo固定，已安装进程内信号处理器，未向真实控制台发信号；不声称原生Ctrl+C全链路验收。CLI完整真实配置、上报自发消息、notice、恢复补读和宿主验收仍未完成。
信号接口参考维护者文档 https://docs.rs/ctrlc/3.5.2/ctrlc/fn.set_handler.html ，2026-10-18通过latest检索命中3.5.2并核对锁定包源码；最初错误版本0.5.1网页访问失败，不作依据。

通知归一化补充：固定产物字段映射group_recall/friend_recall、friend_add、group_increase/decrease/admin，最小kind/时间/消息ID/操作者/子类型，私有通知按实际user_id进入private目标过滤；群事件必须有群ID，拒绝好友事件混入群ID。源码group变更可用user_id/operator_id=0表示解析缺失，输出null不伪造账号；未知子类型/其他通知保持supported=false。2项合成测试覆盖六类通知、私聊过滤、异常字段/时间、未知子类型、缺失身份和不透传URL；不将静态映射视为真实事件验收。

事件输出Schema：独立schema模块，event.watch描述outputFormat=ndjson、逐条responseSchema及单行preStreamErrorSchema和顺序约束。限定ready/message/notice/gap/summary字段，账号/引用字符串格式、未知段标记、媒体引用条件、六类通知与null身份；原消息段/转发节点递归定义不透传原始对象。dev依赖jsonschema精确0.33.0（MIT、MSRV1.71.1），关闭默认HTTP/文件解析，41个新增锁定包均从Cargo缓存离线解析；未完成全依赖审计。独立验证实际合成消息/通知和CLI假会话记录，拒绝数字ID/账号、额外私密字段和缺失可下载引用，启动前错误单独验证。1项新Schema测试及原事件/CLI测试加强；完整命令的领域schema仍未完成。

七个读取命令Schema：contact.list/get、group.list/get/members、conversation.list/get发布实际领域记录、作用域、分页complete/stopReason/coverage/游标约束及统一错误信封，不再返回空data schema。2项新合成测试逐命令调用实际读取/集合内核后独立验证，拒绝数字账号、额外私密字段和未完成页缺游标；group get后端目标不一致返回协议错误，最近活动秒时间拒绝毫秒/超范围。完整输入、其他命令输出和真实读取回归仍未完成。

附件快照内核：显式本地非空普通文件先校验再创建私有临时副本，客户端上限128MiB，沿用逐块截止/长度检查/SHA-256。逻辑指纹仅kind/展示文件名/大小/摘要，不含源路径或随机暂存路径；源文件后续变化不影响副本。未进入潜在写阶段可RAII清理，retain显式保留供未来durable意图绑定，未知结果不能自动删除。2项合成测试覆盖稳定指纹、源变化、副本释放/保留、空/缺失/目录及过期截止零状态初始化。该内核不验证图片格式/后端能力，该准备步骤本身不等同完整CLI附件验收；retain已有签名request/scope所有权，24小时保留期与journal清理已实现，配额与完整写意图关联仍须在实际写入前完成。

附件持久所有权补充：retain在状态锁下签名a1元数据并原子落盘标记，然后保留目录；包含请求键、实例/账号作用域、受管目录、kind/文件名/大小/摘要，无源路径或正文。load仅接受attachments直接子目录，校验签名、请求/作用域/目录、准确两项目录清单、完整SHA-256与长度/截止；额外文件不接管、不删除。保留对象字段私有，只公开受检路径/指纹；准备拒绝与工具标记重名输入。2项新增合成测试验证从磁盘重建、错请求/账号、修改数据、外部目录/额外文件/伪造标记拒绝。此处关联的是所有权元数据，不等同ledger已建立pending；发送前仍须把同一内容绑定写意图并落实配额/租约及未知结果保留。

自有附件缓存清理：media prune为离线入口，无缓存时零凭据/状态初始化；有缓存使用原签名key和状态锁，--instance仅过滤缓存所有权，不要求QQ实例运行。仅request.scope与签名所有者一致、outcome accepted/rejected且24小时以上才删除；pending/unknown、缺失记录、未来时间、无签名/篡改/额外文件保留。先原子写独立删除journal，再逐项删除已核对文件/标记，最后非递归移除空目录并清journal；journal支持文件或标记已删后的重试。扫描上限256个目录名、每目录≤3项检查，超限明确失败；不调用NapCat清缓存或删QQ文件。3项合成测试覆盖年龄/结果/实例过滤、模拟部分删除恢复、额外文件保留、空缓存离线及输出schema。完整配额、所有阶段崩溃/并发故障与附件写关联仍待完成；a1引用元数据不作为附件正文清理目标。

附件配额补充：私有附件缓存默认1GiB/最多256项；准备在状态锁内检查容量并复制，直到已完成副本发布才释放锁，避免并发超额。所有当前普通文件计入占用，目录额外预留至少1KiB标记空间，未知成员不删除/不接管；嵌套或超出有界清单明确拒绝，配额不足返回CACHE_QUOTA_EXCEEDED并指向media.prune，不自动删除unknown材料。2项library合成测试以小预算注入验证并发仅一项成功、未知既有内容计入且保持原样；未采用真实大文件或QQ缓存。配额只覆盖此附件根，其他媒体缓存、全故障矩阵及写意图接入仍待验收。

附件写意图集成：send/reply接受image/file显式路径，最多合计16项；文本/提及后按images/files数组顺序添加本地副本段，仅调用一次send_private_msg/send_group_msg，不叠加upload_*。先准备快照并用稳定逻辑指纹进入ledger pending，随后在同一状态锁下签名保留，所有权额外绑定intent payloadHash，恢复/清理核对该关联。dry-run准备临时副本但不登记意图/保留；同键已知回执不再探测后端，内容变化返回冲突，响应丢失保留unknown及副本。中间传输无独立观察，receipt.fileTransfer=not_observable（最终messageId已接受）或unknown（写结果不确定），绝不据此重传。2项新发送合成测试验证单action/落盘顺序/关联/unknown防重/源变化/dry-run，原回复测试增加附件及重启缓存；尚未完成私聊/混合段真实支持、图片格式矩阵、真实回归（send/reply已有下述假会话）。

send完整CLI假会话：1项Windows测试执行私聊/群×接受/不确定响应四组合，实际CLI通过随机临时凭据访问loopback假HTTP，检查text/image/file段、私有副本字节与pending先于唯一send_*。关闭服务后同键离线命中，request get返回确定/unknown及附件状态，副本不暴露为路径且unknown材料保留；最后清理测试凭据。公共HTTP夹具复用有界握手/请求体/等待，不复制QQ数据。已绑定写错误的meta.scope来源于原请求，unknown.nextAction为request.get及request_id，与输入schema独立验证；错误scope装箱避免扩大常见Result。真实QQ、图片格式、真实reply协议及窗口回归仍待验收。

reply完整CLI假会话：与send共享有界HTTP/子进程夹具，新增1项Windows测试覆盖私聊/群×接受/不确定四组合。临时签名m1包含原账号/实例/run与消息身份，登记第二实例但不创建其凭据；reply不传conversation/instance，自动推导正确实例并通过get_msg复核原消息，回复段在文本/图片/文件前，仅一次send_*。服务关闭后同键和request get离线读取；显式other-instance返回INSTANCE_CONFLICT而不尝试其不存在的凭据。全部材料合成，未连接QQ；这不证明图片格式/混合段或真实会话支持。

写入/请求输出Schema补充：send/reply区分dry-run和accepted，描述请求键、原作用域、缓存标记、消息接受与附件指纹/传输证据；request get描述有界操作标识、四状态及已知消息回执/不确定附件/预检拒绝记录，未实现运行操作保留独立扩展边界。实际CLI和dry-run记录用独立验证器校验。receipts模块拒绝缓存接受回执缺失、数字/零/溢出ID、额外字段或与重试目标不符，绝不把损坏记录作为新写；后端无有效i64/u64非零消息ID仍unknown。ledger读取检查作用域/请求/操作/摘要格式。1项新增合成测试覆盖缓存破损零后端调用，原测试扩展ID边界；其他命令Schema和真实协议仍待完成。

媒体字节发布内核补充：copy与base64共用有界流写入/临时文件/哈希/不覆盖发布；严格标准base64、解码后≤128MiB、可用大小相符、总截止，guard在发布前执行且失败清临时文件。2项新增合成测试覆盖合法字节、哈希/覆盖保护、非法填充/非base64/长度不符和身份变化不发布；未调用真实媒体API。固定bundle查明NAPCAT_WORKDIR/cache与NapCatTempPath不同，QQ下载路径来自内核事件，见 FACT-001/RFC-0003，不能把前者当所有QQ媒体授权根。API字节提供、独立传输预算与自动配置仍未接入，media download完整能力未验收。

媒体专用字节传输与条件入口补充：media_transport独立借用反序列化base64/大小，get_file/get_image编码≤128MiB解码对应上界，额外元数据沿用普通响应预算，其他API仍≤4MiB。缺失base64返回MEDIA_BYTES_UNAVAILABLE，异步/非最终成功拒绝；忽略file/url，不读取后端路径或跟随URL。media download推导f1所属实例，先验证输出不覆盖、重读原消息/附件，再仅以显式file_id获取字节；已知大小冲突、严格解码错误及最终身份变化不发布。1项编排测试覆盖这些分支，2项回环假HTTP测试验证大于普通4MiB的合法媒体及独立元数据限制；未跑128MiB峰值内存或真实媒体。

来源限制：固定产物图片只给可覆盖的文件名别名，message元数据为downloadable=false并在下载时返回MEDIA_BINDING_UNVERIFIED，不调用媒体API。file显式file_id可进入条件路径；不声称该ID为密码学来源证明。enableLocalFile2Url默认关闭，自动配置未实现；图片可靠来源仍为RFC-0003待决目标，不把有字节等同来源正确。

媒体完整CLI假会话：1项Windows测试覆盖私聊/群，以随机临时凭据/签名引用和loopback假HTTP运行实际子进程；多个实例下从f1自动选择，显式冲突零域读取，get_msg/get_file参数及17次有界请求核对，返回字节/大小/作用域通过独立Schema校验，已有输出与离线重复不覆盖。测试结束清理临时凭据，未访问真实QQ；完整图片/配置/缓存/故障矩阵仍未完成。

消息/历史输出契约补充：schema模块抽取message/segment/node公共定义供get/history/search/event复用，后续类型变更不需重复维护。get要求作用域及完整消息对象；history/search限定成功页、含data的PARTIAL_RESULT及普通错误三分支，未完成必须h1续传，完成不得含游标且停止原因为backend_end/range_satisfied，coverage仅backend_accessible。既有消息/历史测试增强独立验证实际转发、私聊/群续传、搜索、扫描中断和拒绝等输出，并拒绝数字ID/账号、额外正文、缺游标、虚假全量覆盖及部分结果缺错误。130项测试数量未增加；其余命令领域Schema及全输入约束仍待完成。

总截止贯穿补充：Command集中读取既有timeout参数，dispatch入口一次确定Instant；实例/凭据/引用准备、集合/历史/导出/媒体/文本附件准备/发送及doctor接收同一截止，不在领域阶段重新起算。collection/export/read/doctor保留独立调用包装，新增execute_until/diagnose_until供入口继承；Session在身份检查前后和领域操作边界检查时间，媒体/导出在输出/锁准备前检查。2项新增合成测试核对12种带timeout命令及默认值，过期时零后端调用、零引用/锁/输出创建；既有诊断测试增强过期零API/零状态。完整脚本132项/92源码包文件通过后，doctor补充另通过clippy、6项deadline/diagnostics定向测试及release/包检查。同步OS调用不能保证立即中断，完整并发/文件系统故障与真实协议仍待验收。

本次上下文收尾：编号唯一/Markdown与JSON证据引用、相对文件链接、公开文档敏感信息扫描、TASK-002的38/71计数及总览交叉核对通过；git diff --check与Semifold配置检查通过，8个changeset计划为0.1.0→0.2.0，仅查看未消费；暂存区为空。未启动、登录、注入、读写或停止真实QQ。

图片来源定位补充：根据FACT-001固定GetMsg/getImageUrl/marketFaceElement证据新增media_source纯解析模块，无网络请求。限定三种明确HTTPS主机、精确路径及appid/fileid/rkey/spec字段，URL≤16KiB、资源字段≤4096字符；拒绝认证/端口/fragment、未知或重复查询、路径归一化绕过、无效或过长资源。持久摘要含主机与资源身份，不含rkey；rkey轮换不改变同一资源摘要，主机/资源变化保持拒绝而非自动换来源。

f1元数据新增可缺省的原消息段位置和来源摘要，归一化时仅存摘要；复核同时检查位置、类型/ID、已知名称/大小/来源。没有位置的旧引用仅可唯一匹配，多个同名候选明确拒绝。3项来源解析测试和1项消息编排测试覆盖这些形状、同名不同资源、位置交换、资源变化/缺失、rkey轮换、旧引用歧义/唯一升级及引用文件无URL/密钥。原文件media download CLI假会话回归通过；downloadable对图片仍为false，未增加TLS依赖或执行CDN请求，完整传输与真实协议待完成。

来源定位阶段完整脚本fmt/clippy/136项离线测试/release/源码包白名单94文件通过；TASK-002当时为36/69，图片来源定位作为已验收子项单列，完整图片任务仍未勾选。保持未暂存、未提交，不修改NapCat或真实会话。

图片HTTPS与编排补充：新增media_https，独立原生TLS客户端，使用系统证书/主机名验证，不接收OneBot凭据，不启用Cookie，显式no_proxy/禁止重定向和自动重试。请求前重核来源摘要及总截止；只接受200，声明类型仅image/*或application/octet-stream（缺失可保留为未知），拒绝压缩编码、零/超128MiB已知长度，至少读取一字节后返回有界流。落盘复用128MiB/大小/截止/SHA-256/不覆盖发布，读取跨过截止不继续写入；最终身份必须与原作用域账号一致，包括登记账号尚为空时。

verify_source保留新鲜get_msg的对应元素URL于内存，独立检查段位置/ID/已知属性与来源摘要；不使用get_image文件名映射。历史未带URL或只有本地路径时，downloadable可表示允许一次受作用域保护的解析，下载从新鲜原消息恢复受检来源，绝不读取该路径；已有不受支持网络URL不广告为可下载。无可靠来源仍MEDIA_BINDING_UNVERIFIED。该标记不保证远端内容仍可用，也未验证实际图片格式解码。

4项新增library测试使用原创测试证书、每客户端测试信任根与内存rustls回环服务：合法TLS字节/无Authorization、Proxy-Authorization、Cookie、Referer；不受信任根/主机名不符拒绝且错误无URL/rkey；302/403、HTML、压缩编码、超限长度和总截止拒绝；私聊/群×成功/最终切号，使用省略URL或本地路径的历史引用，再从新鲜消息解析图片，经实际TLS流到原子输出并独立验证Schema。TLS服务使用内存密钥，不安装全局信任；初次原生TLS服务端夹具有链导入问题，已替换。其5个临时测试密钥容器按公钥与原创夹具精确匹配后清理，没有枚举或删除不匹配项；最终代码不再创建这些容器。测试证书/密钥仅为公开测试夹具，不用于产品信任。

依赖：reqwest 0.13.5启用native-tls-no-alpn，native-tls 0.2.18（MIT OR Apache-2.0，MSRV1.80）、hyper-tls 0.6.0及schannel 0.1.29（MIT、MSRV1.71）；20个新锁定传递包，仅获取Cargo依赖。dev rustls精确0.23.46（Apache-2.0 OR ISC OR MIT、MSRV1.71），仅std/ring/tls12；ring 0.17.14（Apache-2.0 AND ISC、MSRV1.66），15个新锁定包从缓存离线解析。生产图片客户端使用平台TLS，rustls仅供测试服务；未完成全部依赖许可/安全审计。

完整离线脚本140项测试/release/98源码包文件通过；随后历史省略URL适配通过4项TLS、19项消息/历史/事件Schema及2项Skill定向回归、fmt/clippy和release/打包。Skill Creator静态校验通过。完整图片CLI/真实CDN、代理环境完整矩阵、流中断全故障、格式能力和托管开关仍待验收；不把本次结果升级为完整媒体或账号安全保证。

托管配置生成补充：依据FACT-001固定ConfigBase/OB11/WebUI/NapCat字段与初始化观察，新增runtime_config模块，原创建构通用onebot11.json、webui.json、napcat.json。仅审核版本与三个不同≥1024端口；全部host=127.0.0.1，业务HTTP/WS共用256bit随机token，管理token和JWT secret各自独立256bit，秘密类型不提供Debug/Serialize。HTTP禁止CORS与附加WS，独立正向WS启用array/自发事件；反向HTTP/WS、SSE、plugins为空，文件字节开关开启，转发自动展开/图片代理/调试/文件日志关闭。WebUI无账号预设、回环白名单、不信任转发IP头；JWT只经显式子环境回调，避免会打印覆盖值的管理token环境变量。

该模块不打开文件、不启动进程或操作凭据；ConfigFile只能向调用方明确提供的Write写内容，私有ACL/所有权事务尚由后续实现承担，不能据生成模板宣称已保护真实运行材料。三个端口只验证数值/互异，不证明空闲或属于受管PID；原版WebUI回退/日志行为不因模板而消失。未改变Hook/Bypass未核实字段，也不推断首次登录、账号配置继承或启动兼容已通过真实原版验收。

3项新增合成测试核对回环/独立凭据域、未预设账号及无反向网络、消息/事件/字节能力、JWT不进入JSON、独立生成不复用秘密、写出错误脱敏，以及不支持版本、低端口/端口冲突拒绝。未引入依赖、未执行上游或访问真实配置。完整脚本fmt/clippy/143项离线测试/release/100源码包文件通过；TASK-002为39/72，私有落盘/凭据事务和实际监听/进程绑定仍未完成。暂存区为空，未提交。

私有运行材料准备补充：runtime_store仅接受登记匹配的实例/run、安装版本/摘要快照、计划HTTP/WS端点和业务凭据键，且阶段为preparing/prepared、账号/进程记录为空。状态锁内先保存内部r1签名所有权日志，再在受保护条目保存绑定同一引用的整组API/管理/JWT秘密；业务token条目从该组幂等恢复。若首次已有未知目录/条目、已有材料却丢失秘密、引用/配置不符，则保留并拒绝，不重新生成已保存秘密。凭据写入返回失败时，下次先读取已提交值；未曾保存且未产生材料时才可重新生成。

私有暂存与最终目录由签名日志认领，路径限当前状态根的本地Windows驱动器子目录；不跨卷，不接受UNC。根最多检查7项、config最多4项，每文件读取≤16KiB；合法完整布局为三份配置、五个受管子目录及所有权标记。新文件先施加私有ACL再写出；中断写仅允许准确预期前缀，在同一受管暂存文件句柄追加缺失后缀，不改写已有字节。额外/非前缀/链接拒绝，已发布文件必须完整匹配。目录发布前后核对准确布局，发布后才提交prepared；中断后可复用同一日志/秘密和已发布目录完成登记。

原生发布使用已有winsafe 0.0.29的安全封装MoveFileW，不依赖std::fs::rename可能变化的平台实现。依据[Microsoft MoveFileW文档](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-movefilew)，目标名称须不存在，目录须同驱动器；检索2026-10-18，可变官方来源，不作NapCat固定复现基线。新增1项本机Win32合成测试分别验证既存文件/空目录目标均失败，源及目标字节保留。非Windows在写入前返回不支持，未声称其他平台的发布语义已验证。

5项新增Windows材料测试：随机临时原生凭据命名空间的准确两个条目，确认缺失后才取得清理责任；prepare及重复调用复用原秘密、配置逐字节核对且注册表无token；内存凭据注入两种条目的写前/写后失败；模拟暂存缺文件/中断前缀、发布后登记前恢复；未知目录/条目、额外/修改材料和丢失秘密保留；错run/运行阶段/PID或账号已登记/端点不符及过期均在凭据访问前拒绝。原生测试只写/删自身确认创建的两个条目，不枚举其他凭据。

准备态业务隔离：Session身份检查、原生doctor API探针及事件入口拒绝非ready；普通读取在解析凭据/创建签名状态前拒绝。status仅输出登记阶段、account=null、identityVerified=false，不访问计划端点。新增1项实际CLI测试用自有回环监听陷阱和缺失凭据键，核对status、联系人/历史/事件及下一步Schema，确认零TCP连接和零引用初始化；既有deadline测试增强零后端调用。send/reply假会话加强停止态离线缓存/request get的八组合，未因就绪门禁重放。ready登记本身仍不证明实际PID/监听所有权，真实绑定仍未完成。

完整脚本fmt/clippy/150项离线测试/release/102源码包文件通过；首次并行链接曾暂时无法打开MSVC库，单编译任务重跑通过，最终默认脚本亦通过，未改全局构建设置。随后发布后复核/指引更新通过5项材料、1项原生发布及2项Skill定向验证和构建/打包，补充的原生清理责任/CLI监听陷阱检查各通过定向回归。Skill静态校验通过。TASK-002为40/73；实际启动意图、PID/监听绑定、运行后配置/升级/退役和完整跨进程杀进程/落盘故障矩阵仍待完成。不新增QQ会话操作，不改NapCat，不暂存、不提交。
