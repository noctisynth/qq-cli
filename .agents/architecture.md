# Rust 工程架构与维护入口

文档编号：DESIGN-002。状态：Active。更新：2026-10-28。
产品契约见PLAN-001，重构决定见RFC-0005，质量验收见TASK-003。

## 从任务定位代码

| 子系统 | 职责与入口 |
|---|---|
| app | 组合用例：management路由在administration，实例/引用解析在selection，状态确认及待登录观察在status，启动检查点在runtime，消息调用在messages，事件入口在watch，受管客户端组合在backend |
| cli | args定义命令/参数，validation做无I/O语义校验，registry/schema提供机器契约，skill提供任务指引，stream_output管理进程输出 |
| core | 共享作用域/会话标识、错误、截止和结果信封；不创建进程、凭据或客户端 |
| storage | 配置、本地状态锁/原子落盘、系统凭据和签名引用，不路由命令 |
| requests | 消息意图ledger、跨消息/启动的keys互斥、query离线查询 |
| messaging | 联系人/群/会话、消息/历史/导出、发送、事件用例；attachments封装副本/配额/清理，media封装引用解析、下载与发布 |
| napcat | install封装固定包获取/审计/登记/卸载；compatibility/doctor/qq定义兼容、诊断和QQ安装观察；control封装独立管理鉴权及过滤后的登录观察；runtime封装预留/准备/一次调度/恢复 |
| native | 只读宿主/进程能力、Windows Job及凭据互斥原生边界；unsafe精确限job、credential_lock、macos_process、macos_socket、macos_terminate、macos_acl、macos_xattr、macos_copy、macos_signing_info和macos_disk；凭据互斥不处理凭据值或业务逻辑 |
| transport | OneBot动作/响应、受检HTTP、WebSocket和媒体字节封装；不依赖消息用例或应用路由 |

src根层只保留lib.rs和main.rs。lib.rs声明以上边界，main.rs处理参数解析、退出码和标准流。内部Rust路径不是稳定SDK契约；公开CLI契约由PLAN-001和schema界定。

平台隔离（RFC-0008）：Windows Job 与凭据互斥的物理实现和原生测试位于 native/windows；native/mod.rs 通过 cfg(windows)/path 选择，逻辑模块仍为 native::job 和 native::credential_lock。Windows unsafe 只在这两个模块声明处允许，不扩展到整个 windows 目录。进程观察的平台编码见 SPEC-008；macOS进程、凭据、发现及运行编排由下述平台模块承担；实现范围与验收差距见TASK-006，目录隔离本身不作为运行证据。

## macOS 平台模块

通用runtime和install各只有一个cfg门控的macos入口，不在通用目录增加macos_*同级模块。runtime/macos按职责组织adapter、creation、layout、config、material、assembly、deploy、start、run、compatibility、stop、restart和maintenance，release维护平台版本目录；准备、部署、运行、停止及版本维护各自封装，不将公开CLI解析下沉到平台模块。
install/macos只处理source、bundle和signing：固定来源/镜像管理、受控应用树和签名策略。worker_entry统一分派该层私有工作进程，main及独立装配宿主复用同一入口。原生SDK能力继续留native/macos，unsafe仅在native/mod.rs明确列出的十个能力上放行；结构脚本从同一显式审查清单派生数量，不能用目录整体放行。
macOS自工作进程宿主定位集中在native/process/current.rs：从本进程内核镜像取得实际可执行文件，复制、签名、SDK校验及磁盘工作进程共享；可切换的安装入口不作为宿主权威，不新增unsafe边界。

上述Rust模块路径是内部边界，不保留旧macos_*兼容别名；CLI、状态/签名记录、固定资源字节及平台支持范围保持原契约。历史CHECK中的路径按当时提交解释，当前代码导航以本文件为准。

## 应用调用顺序

app入口先做参数语义校验；skill/schema保持无配置I/O。安装固定版本目录检查先于来源、凭据和状态初始化，许可注记仅作信息。管理命令与聊天会话分别路由。

聊天调用由selection选择明确实例或从签名引用推导作用域，保留原有“先拒绝歧义/冲突，再访问相关凭据”的顺序。backend只从签名运行能力构造受管HTTP/WS。status是显式确认实际账号的入口；消息用例不能借此自动承认切号。

启动内核返回Started和运行状态，不构造CLI登录提示。app/runtime把待登录结果转成AUTH_REQUIRED，复用就绪结果共用同一输出函数；账号仍须经过Session核对。具体系统/凭据/API诊断适配器位于app/diagnostics，napcat/doctor只依赖Probe契约，避免生命周期反向调用应用组合层。系统安装发现位于napcat/doctor/discovery，受管macOS副本观察位于app/diagnostics/macos，经Probe的显式只读报告接回通用诊断，不在核心入口复制平台诊断流程。
status/pending仅组合已有原控制能力、原秘密和固定管理协议；状态事务在网络调用前释放，返回前再核对实例/安装快照。其loginObservation只作进度，不更新账号或ready；原账号确认仍在status的OneBot用例中，消息调用不能借控制面绕过会话门禁。

## 生命周期与传输的内部边界

napcat/runtime/run_job的公开入口在mod.rs，能力类型和不变量在同一入口可见。journal处理签名记录，launch处理一次创建与原根恢复，service处理API归属/发现/恢复，prelogin处理登录前保留；内部文件不作为额外公开命名空间。

macOS服务恢复位于napcat/runtime/macos/run，以独立j2记录和原启动回执恢复原服务Pinned；journal封装原记录认证，publication只接受原派发进程，scope声明固定继承执行范围；app/backend按平台选择受检HTTP和SocketSource。该模块负责j2生产及已有记录的只读验证；首次创建由start的s1事务显式调用publish，控制端点只取认证记录，不隐式发现任意服务；不得从公开PID或端点构造替代运行权威。

runtime_store的准备/复核仍保持同一状态事务中的显式顺序；输入信任检查、配置准备、持久意图和原生创建分别位于startup、runtime_store、start_intent和dispatch/run_job。不能为缩短函数而把锁顺序、持久化先后或未知结果边界隐藏在通用容器里。

媒体字节封装属于transport/media；消息侧的文件发布与引用/账号守卫属于messaging/media。出站影本仍由messaging/attachments管理配额和所有权；macOS运行前缀由runtime_store统一命名，start/sandbox/history从g2及完整p2派生同实例已停止祖先，sandbox只开放当前及这些祖先影本的只读访问；固定来源链使后续发送或缓存清理不改变启动输入，不在平台层重建上传或消息意图逻辑。两层共享预算来源，底层不导入消息用例。Job内核只处理原生句柄、成员和连接归属，上层保留事务不下沉为原生策略。
受检HTTP在transport/owned_wire内私有共享hyper实现与连接归属、预算和响应后守卫；transport/owned_http包装OneBot协议，napcat/control/owned包装固定WebUI路由。控制鉴权不使用账号API token，控制状态不产生账号身份或登记ready。协议模块和传输模块不互相反向依赖，运行恢复仍负责提供签名来源的完整身份与固定端点。

## 修改与验证

新增能力先选定所属职责；跨子系统编排放app，数据/协议规则放对应子系统，不在lib.rs添加旧根路径别名。单元测试放所属模块tests文件，集成/CLI测试在tests目录；测试中的高层组合依赖不扩散到生产实现。

scripts/check-architecture.ps1检查根入口、允许的子系统、应用入口审查预算、底层到应用/CLI的反向引用、传输到消息用例的引用以及unsafe允许位置。它是结构守卫，不能替代Rust类型检查、人工职责审查或行为测试。

scripts/check.ps1把结构/契约检查、发布包与版本锁合成检查、fmt/clippy、完整离线测试、release构建与源码包白名单串联。目录迁移还须核对include资源相对路径和独立进程测试的精确过滤路径，不能只看单个编译目标通过。

命令解析、帮助和输入Schema使用Clap定义与共享语义规则；Skill为手写Markdown的编译内嵌产物，命令路由有自动覆盖检查。RFC-0006/TASK-004记录已完成的单源契约整改，不恢复手写输出Schema或仅供Schema使用的结构副本。

实施状态（CHECK-058）：普通JSON和NDJSON均由实际Serialize + JsonSchema类型生成，无通用响应、LegacyWire、success(Value)或手写输出Schema。Clap实际类型提供输入字段约束，默认/必需/标志/冲突直接取Clap，条件依赖和组合预算由实际校验与Schema投影共享规则。Schema文档和原始协议、私有规范化指纹是受控JSON边界；不得新增任意JSON公共领域字段。运行权限、文件/签名、在线身份、UTF-8字节及时间先后仍需实际验证。

自动化见SPEC-005/RFC-0007：可复用Windows质量workflow执行同一本地入口，Semifold负责CD。scripts/package-release.ps1只组合Cargo元数据/固定目标构建/版本确认，release.psm1封装可独立验收的显式文件集打包与校验；check-release.ps1使用临时合成PE、ZIP及独立workspace验证保护和锁同步。scripts/release_assets.py只汇总同次质量运行的双平台包，验证版本/原摘要并生成公共校验清单；不构建、不执行或上传载荷。内部架构和完整计划只在.agents，不进入源码包或发布ZIP。

macOS 只读进程能力位于 native/macos/process.rs，以 native::macos_process 精确条件编译并开放独立 FFI 边界；native/process/identity.rs 保持安全 Rust 的平台记录解析，方案见 SPEC-008。

macOS 已连接 TCP 归属通过 native/macos/socket.rs 和原创 socket.c 的窄 SDK 适配查询；只读指定进程描述符，不扫描全局进程。transport/ownership 按平台守卫同一 TCP 流，HTTP/WebUI/WS 协议复用，运行日志仍须提供独立认证来源。build.rs 只在 macOS arm64 编译该 C 适配，产品主体仍为 Rust。

定向终止单列 native/macos/terminate.rs 与原创 terminate.c，仅通过原 audit token 发信号，保持只读进程观察模块不拥有终止方法。私有 Target 可进入签名日志，上层仍负责停止意图、进程树归属与完成确认。

macOS QQ静态审计位于napcat/qq/macos，Mach-O有界解析单列macho.rs；复用Application契约与ASAR成员检查，不执行上游载荷、不开放unsafe。选定文件指纹不替代完整.app签名或运行权威。

官方QQ静态签名位于native/macos/signature.rs，通过security-framework/core-foundation安全封装验证来源要求、严格资源和嵌套代码；不新增unsafe或凭据条目查询。qq::macos::inspect_original组合签名前后选定指纹，保留普通inspect的纯形状边界。

签名隔离位于native/macos/signature/worker.rs；main仅在公开参数解析前分派私有只读worker。父端持有直接Child与存活管道，子端运行同步签名API和独立退出观察；不复用到QQ持久生命周期，也不扩展公开CLI命令面。

macOS应用树清单位于napcat/install/macos/bundle；保留目录描述符枚举和打开，attributes子模块通过节点描述符计算扩展属性摘要，包含可枚举资源叉/quarantine且不追随链接。清单用于来源重核/副本比较，不负责复制、签名或会话；native/macos/acl.rs单列描述符ACL快照及明确owned ACL清除的unsafe边界，以SDK外部表示摘要参与清单；st_flags参与来源复核和副本比较。native/macos/xattr.rs仅包装描述符list/get，固定显示压缩属性；不扩大到权限绕过或写入。macos/bundle/copy.rs管理私有暂存、原描述符复制、完整清单核验和无覆盖发布；native/macos/copy.rs只封装同步fcopyfile及截止回调。native/macos/copy/worker.rs及wire.rs以安全rustix封装匿名socket/SCM_RIGHTS能力传递，持久子进程只做描述符复制；父进程保留guard、核验和发布，隔离入口等待子进程回收后再发布。main分派私有worker，不增加公开命令。macos/bundle/harden.rs仅消费受控复制返回的内存能力，在持有节点上清除ACL/设置私有模式，再核对整树内容和身份；普通扫描不能授予修改权。完整部署由assembly/deploy接合；父端文件系统预算、失败现场写权限恢复和清理的剩余范围见TASK-006。

副本入口部署由runtime/macos/adapter/package.rs复核Prepared对应的适配资源并组合install/macos/bundle/deploy.rs；安装层只处理固定package的原摘要、受控写能力、描述符写入及全树差异核验，不反向依赖runtime。失败消费能力并保留现场，后续签名/生命周期独立完成。

macOS副本签名策略位于napcat/install/macos/signing.rs，生成固定角色权益并组合严格静态签名校验。native/macos/signing_info.rs是独立只读unsafe边界，仅SecCodeCopySigningInformation和CF所有权/类型转换；不执行签名、不访问Keychain条目、不信任codesign文本输出。结构门禁精确允许十个原生模块，其他模块继续禁止unsafe。此策略观察不授予部署/启动权威，嵌套目标角色映射、签名执行器和整树绑定仍由安装编排完成。

macos/bundle/signing.rs在完整清单上逐项发现Mach-O与code bundle，显式匹配角色元数据并构建从内到外的签名计划；复用qq/macos/macho的有界原生slice解析，不增加解析器副本。macos_signing_info补通用签名系统元数据只读提取及有界CFPropertyList解析/所有权转换，仍无签名效果；计划的根/指纹重核不替代活跃副本写权限。

Mach-O重签名载荷摘要位于qq/macos/macho/signature.rs，复用父模块thin/fat布局，保全全部既有slice的非签名内容。signing计划持有该私有摘要并提供独立verify_payloads；它与整树Inventory指纹、系统签名/权益验证职责分离，不授予写权限或部署完成证明。

签名后的整树变化比较位于macos/bundle/signing/transition.rs，只允许计划映像与精确bundle封印文件的签名变化及其父目录时间变化；再调用载荷保全。原封印缺失拒绝，不授予副本写权限，不替代签名权益检查或部署事务。

macos/bundle/signing/execute.rs消费活跃加固副本，组合固定codesign直接Child、匿名普通文件权益输入、逐目标整树变化/载荷及Policy验证。同步sign_copy仅作协作式内核和合成对照；生产装配调用下述sign_copy_supervised，组合隔离校验、写入监管与同步后再交d2记录完成。

macos/signing/worker.rs在安装层解析固定角色并调用同步Policy，native/macos/signature/worker.rs复用只读工作进程管理和看门狗；native不引用Policy或安装编排。sign_copy_supervised组合校验隔离与原树守卫，main只分派私有入口；codesign写入进程监管独立完成。

macos/bundle/signing/supervisor.rs持有codesign直接Child，复用execute的有界回收；父端通过EOF请求取消，不先杀监管者。main分派私有入口，继承FD绑定精确目标与匿名权益。sign_copy_supervised组合写入监管、只读权益隔离与原树守卫；持久恢复和父端文件系统预算仍独立。

签名中间步骤复用macos/bundle私有inspect_using：完整stamp不变才复用摘要，仍枚举/打开全树并计入内容/属性预算。transition的verify_step只接受当前目标变化和原载荷；最终公开完整核验不使用缓存，owner不能从缓存恢复。

macos/bundle/signing/sync.rs在签名核验后，通过当前清单描述符同步精确文件、受影响目录和原私有父目录，最后执行设备刷新。每次调用前后检查guard/stamp；失败不返还复制能力，不提供磁盘恢复授权。

实例部署日志位于runtime/macos/deploy，以原生命周期锁和短状态事务发布d2 Pending/Complete/Unknown；复用runtime_store路径身份，不混入NapCat安装包登记。macos/bundle/artifact仅从已签名同步的活跃副本消费生成持久观察，恢复核对身份/摘要但不恢复owner。state的lock_identity仅供同次操作跨短事务检查原锁对象，不是可序列化锁权限。References识别部署索引以阻止遗失原密钥后重新生成权威。

install/macos/source组合完整Inventory与隔离原始签名检查，绑定固定官网载荷和原树指纹，提供只读来源观察；不恢复复制owner，不创建进程或绕过运行意图。固定DMG描述不等于下载/挂载回执。其roles子模块维护固定官网43个bundle的显式角色，Source提供原树计划和副本重建所需元数据；PluginHost由安装权益策略及两个私有worker精确处理，不从文件名自动推导。

macos/adapter/publication以AdapterPlan统一纯内容规划、发布和校验；package将纯改写计划与旧发布验证入口分开，生产元数据由Source.package_bytes经Inventory固定路径描述符读取。安装层仅提供认证字节，不依赖运行层；完整部署摘要/意图组合仍由运行编排负责。

runtime/macos/assembly负责固定Source、i1安装认证、准备路径、入口计划及角色摘要的输入生产，并在d2回调内组合隔离复制/加固/入口部署/签名。inputs是私有确定性摘要结构，kernel是调用者负责认证的低层效果序列；Plan不创建账号或运行权限。独立合成宿主tests/probes/macos_assembly.rs注册真实私有worker，macOS门禁只运行synthetic模式。

运行目录准备位于runtime/macos/layout，复用macos/adapter固定目录集合，以l2意图和描述符目录身份提供只读Layout；macos/assembly消费该凭据并绑定输入摘要。其profile.rs从认证后继来源和原停止证明选择同实例QQ数据所有者，原引用和目录身份由l2统一绑定；不另建凭据或复制数据。fs仅处理固定目录创建/校验，记录层负责原锁、实例快照和不重放；不创建账号、运行或清理权限。

首次配置文件效果位于runtime/macos/config，消费l2 Layout与runtime_config::Bundle，在napcat-data/config描述符目录中无覆盖发布默认配置，返回无明文凭据的身份/摘要观察。它不承担Keychain访问、持久意图或实例状态提升，生产接合须由上层准备编排提供认证和guard。

runtime/macos/material以mod保留首次准备及状态提升的显式顺序，journal封装r2签名记录读写，recovery封装只读恢复与启动前校验，secrets封装凭据读写和秘密摘要。runtime/macos/material编排r2一次准备意图、原始凭据包摘要、macos/config发布及prepared状态提升；secrets隔离凭据序列化/恢复，不把明文写入签名记录。原生命周期锁与短状态事务复核作用域，恢复不重放未知效果；装配、运行凭据观察和启动权限仍独立。

instance_creation共享reserve_in登记协议，原Inputs入口使用n1；macos/creation组合固定Source/i1校验、原生命周期锁和短状态事务后使用n2，避免复制登记算法或跨平台解释旧记录。macOS预约不依赖Windows启动键，亦不授予启动兼容性。

macos/source/image负责固定DMG长度/摘要、原文件与quarantine观察；acquire组合独立缓存锁、私有暂存、固定同源HTTPS和无覆盖持久发布。完整中断文件重新认证后可发布，不完整现场不接管；此层不挂载、不产生Source或运行权限。

image/mounted以固定hdiutil/plutil的有界机器输出及描述符fstatfs组合既有只读挂载观察；报告解析和直接Child查询分别位于report/command，无新增unsafe或依赖。Observation借用挂载后认证Image并持有目录，不产生挂载或卸载权威；系统查询回收与挂载生命周期分离。

image/transition消费挂载前Image，复用macos/bundle的描述符Attributes扫描及原生ACL摘要，只接受同一文件的有界系统校验缓存属性转换并返回新Image。Attributes仅向安装层扩展可见性，不复制属性解析器；转换不执行attach、不替代一次挂载意图或监管。

image/mounted/media使用固定ioreg IOMedia报告与既有有界plist转换，绑定启动会话和只读整盘/卷注册表对象ID；Observation在返回和复核时比较原附加对象，避免仅依赖可重用BSD编号。复用macos_process的只读boot_session，不新增unsafe或依赖；这不是持久挂载所有权。

image/journal把固定私有缓存、元数据转换和挂载观察组合为m2一次意图；record负责签名索引、输入身份及目录发布。长内容/系统观察不持有状态事务，前后复核原状态锁与挂载锁；Complete恢复返回持有根句柄的只读Ready，不恢复效果或卸载权威。生产attach监管和生命周期释放另行接合。

image/attach提供m2生产调度入口并监管独立工作进程；worker以stdin一次许可/存活协议和继承镜像/空目录FD执行固定只读hdiutil。复用mounted/command的有界输出、直接Child回收；父端只关闭存活管道、不终止监管者。完整挂载结论仍由journal独立核验，原生系统辅助进程与磁盘释放不由退出码证明。

native/macos_disk增加窄SDK边界：只读字符设备FD的主/次设备号与属主核对，以及消费原FD的一次DKIOCEJECT。C适配不枚举设备、不接收卸载路径、不开放force/format；install/macos/source/image/mounted在原挂载及IOMedia身份下建立FD绑定。拥有FD不等于拥有挂载，效果前仍由生命周期负责原意图/所有权和guard。

image/journal/release在原m2完成挂载上建立绑定释放请求的一次意图；原根读取者与raw FD重叠固定，意图保存后释放本次根读取者再调用效果。恢复以镜像/挂载映射和原IOMedia消失为证据，不重放；已完成挂载请求返回Released。它保留原记录与目录，不承担生产效果监管或目录回收。

image/release在m2释放意图内把原整盘/卷raw FD及有界元数据传递给独立worker，mounted/devices/transfer复核继承FD与当前媒体代次。worker将标准输出/错误替换为null，随后固定普通unmount和一次原FD eject；不重开设备、不force。image/supervision共享attach/release的许可、父存活、截止与未知退出协议；父端取消不杀监管者，m2独立观察消失才记完成。目录清理与新请求接续仍由后续生命周期处理。

image/journal/cleanup仅处理m2版本2的已完成释放；record绑定原空mount身份，并按输入/请求派生独立父目录。Cleanup三个持久阶段围绕描述符相对rmdir及父目录同步恢复，不递归删除；仅Complete墓碑放行同镜像新请求。版本1沿旧路径恢复，缺失原mount身份拒绝清理；历史请求不检查或操作后续挂载。未知attach/未完成release现场继续保留。

runtime/macos/start是正式启动用例：selection只解析原创建键及受检运行复用，prepare组合既有n2/l2/r2/d2，source界定官方挂载读取/释放范围，launch只处理一次sandbox创建与j2生产，sandbox只渲染固定隔离策略。app/runtime/macos保持参数与响应组合；不将CLI类型引入运行层。原锁释放/重取、身份比较及s1顺序保留在用例入口可见，避免准备模块递归争用同一生命周期锁。material的prelaunch检查在原事务中认证原秘密与精确配置，运行后recover_running只恢复原秘密/目录身份，不把可变配置当初始模板。

macos/compatibility以key封装系统/入口/配置软件身份，local封装独立k2索引，observation只在原拥有运行两次在线核验后发布；start/observation从n2/d2/r2和适配器重建当前v3输入并匹配j2/s1，不恢复写能力。新j2另存原软件键用于预先识别当前组合；旧/不同键仍只读恢复，不用新模板解释旧运行。Key与Windows k1保持独立。native/platform/macos只用安全Rust读取固定系统版本元数据，kernel信息使用既有rustix安全封装，不新增unsafe。start/launch统一组合实际环境与原JWT秘密，并将字面参数/环境/工作目录和软件组合纳入同一Target摘要。

macOS停止位于runtime/macos/stop，journal维护独立p2及原成员父索引，members组合原audit代次的冻结/终止与当前UID受检映像集合，session保持原锁、实例/安装/j2/d2/p2复核和完成先于stopped确认的显式顺序。候选发现不授予成员身份；原父身份核对后保存成员，未知信号不重派。run新记录保存原根audit token和scope，start/launch将其声明绑定v3输入；旧服务仍只读恢复，不补造停止或新兼容权威。底层census/set仅提供当前UID活出生快照和固定文件集合观察，不持有停止策略或写日志职责。

运行登记移除共用runtime/remove与x1事务，平台差异限于停止完成证明和默认创建来源：Windows核对p1/n1，macOS核对完整p2/n2。default_start的退役入口接收当前锁guard，macOS状态层Directory提供受原目录/文件身份约束的单文件unlink及同步，不开放递归清理。instance_creation在原移除索引存在时拒绝首次恢复和后继登记，避免Reserved阶段中断后复活已移除实例。

macOS同安装重启位于runtime/macos/restart，显式组合输入预检、原锁下stop和generation后继认领，再用n2精确请求续接start。generation共享纯代次/登记算法，journal按平台选择g1或g2；Windows版本维护仍独立条件编译，不将其schema2维护字段用于macOS。start/selection从认证来源取得后继键，default_start通过根来源识别后继，不依稳定ID补造请求。

macOS跨安装维护位于runtime/macos/maintenance：preflight负责固定源/目标与兼容/来源可用性，mod保留原锁下v2→p2→g2/n2顺序及锁外精确请求恢复。共同的version_intent区分v1/v2，rollback解析独立在version_intent/rollback；generation的schema2数据协议按g1/g2认证，不复制平台终止能力。来源准备由qq-source.lock协调读者及退役，锁顺序和收尾预算见SPEC-008。
