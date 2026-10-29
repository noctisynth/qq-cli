# 证据登记

文档编号：EVIDENCE-001。状态：Active。更新：2026-10-28。

本表登记每份证据的当次范围；历史状态不是当前能力清单。当前事实见 FACT，当前差距见 TASK。
CHECK 的方法、结果和限制在各自文件中，本索引不复制完整实验摘要。旧记录的授权仅属于当次操作。
历史CHECK中的源码路径、提交SHA和变更集名称按当次版本解释；本地macOS提交线性压缩后，各合并提交正文保留原提交SHA及变更集映射。旧SHA用于标识当时验收程序，不替换为压缩后提交并冒充重新验收；当前技术与任务以Spec/TASK为准。

## 验证记录

| 编号 | 记录 | 当次状态 |
|---|---|---|
| CHECK-138 | [离线接口目录与详情](api-discovery.md) | Offline slice verified |
| CHECK-139 | [受检直接JSON调用与持久回执](api-call.md) | Offline JSON call slice verified |
| CHECK-140 | [账号资料及状态领域任务](account-profile.md) | Offline account slice verified |
| CHECK-141 | [群管理领域指令](group-management.md) | Offline group management slice verified |
| CHECK-142 | [联系人资料与好友关系](contact-relationships.md) | Offline contact slice verified |
| CHECK-143 | [用户点赞及在线状态](profile-likes.md) | Offline user interaction slice verified |
| CHECK-144 | [群资料与成员查询](group-queries.md) | Offline group query slice verified |
| CHECK-145 | [群申请与通知用户链路](group-requests.md) | Offline group request slice verified |
| CHECK-146 | [签名消息撤回与显式已读](message-actions.md) | Offline message action slice verified |
| CHECK-159 | [闪传与有界目录影本](flash-transfers.md) | Offline file-set tasks and directory snapshots verified |
| CHECK-158 | [原生流协议与受检分块文件链路](native-streams.md) | Offline stream/file workflows verified |
| CHECK-157 | [文件目录上传与来源受检链接](files-directories.md) | Offline files/directories slice verified |
| CHECK-156 | [群相册查询上传与显式互动](group-albums.md) | Offline group album slice verified |
| CHECK-155 | [空间发布删除与频道兼容占位](qzone-posts.md) | Offline QQ-space slice verified; guild handlers remain stubs |
| CHECK-154 | [本地自定义表情导入与源文件保留](emoji-add.md) | Offline retained file import slice verified |
| CHECK-153 | [收藏摘要与自定义表情元数据](favorites-emoji-metadata.md) | Offline own-repository metadata slice verified |
| CHECK-152 | [公告与精华快照用户链路](group-announcements-essence.md) | Offline announcement/essence slice and full suite verified |
| CHECK-151 | [群精华、待办与签到](group-message-tasks.md) | Offline group message task slice verified |
| CHECK-150 | [卡片、按钮与复合兼容接口](cards-and-buttons.md) | Offline card/button slice verified |
| CHECK-149 | [消息回应、戳与输入状态](message-interactions.md) | Offline interaction slice verified |
| CHECK-148 | [语音获取与显式转文字](voice.md) | Offline voice retrieval/transcription slice verified |
| CHECK-147 | [单条及合并转发](message-forward.md) | Offline referenced forwarding slice verified |
| CHECK-001 | [初始化验证](bootstrap-verification.md) | Verified |
| CHECK-002 | [Framework 发布包静态审计](framework-audit-verification.md) | Static audit complete |
| CHECK-003 | [原生加载器来源追溯验证](native-loader-trace-verification.md) | Static trace complete |
| CHECK-004 | [Framework 文档与安全结论复核](framework-history-verification.md) | Static research complete |
| CHECK-005 | [Framework 原版运行尝试](framework-runtime-trial.md) | Bounded runtime verified |
| CHECK-006 | [Framework 消息与焦点实验](framework-message-focus-trial.md) | Bounded message and focus verified |
| CHECK-007 | [历史 CLI 方案核对](cli-plan-verification.md) | Documentation verified |
| CHECK-008 | [Rust CLI M0 离线验收](cli-m0-verification.md) | Offline verified |
| CHECK-009 | [指定群历史边界只读实验](group-history-boundary-verification.md) | Bounded pagination boundary verified |
| CHECK-010 | [Agent CLI 完整契约核对](agent-cli-plan-verification.md) | 设计核对完成，业务与真实回归未执行 |
| CHECK-011 | [安装、实例选择与身份 UX 核对](instance-identity-design-verification.md) | 设计/静态检查完成，未运行真实实例 |
| CHECK-012 | [CLI 接口整体审视与职责修订核对](cli-ux-review.md) | 设计检查完成，未实现或实测调用成本 |
| CHECK-013 | [CLI 核心模块离线验收](cli-core-verification.md) | 核心模块定向验收，完整功能未完成 |
| CHECK-014 | [运行前预检与分层诊断验证](runtime-preflight-verification.md) | 元数据/证据判定内核定向验收，真实QQ组合未验收 |
| CHECK-015 | [固定 Framework 预登录监听与保留候选核对](prelogin-lifecycle-source.md) | 固定源码及发布包观察，非真实进程验收 |
| CHECK-016 | [固定 WebUI 登录观察协议核对](control-protocol-source.md) | 固定源码/发布包观察，非真实协议验收 |
| CHECK-017 | [模块职责与应用边界重构验收](maintainability-refactor.md) | 本轮质量整改离线验收，非产品完整交付 |
| CHECK-018 | [许可原文与运行策略修正](license-runtime-policy.md) | 离线核对通过 |
| CHECK-019 | [登录观察实现验收](login-observation-verification.md) | 协议/受检HTTP/只读恢复及公开status离线通过，真实原版待验收 |
| CHECK-020 | [实例维护机制与离线验收](runtime-maintenance.md) | 公开停止/注销有界离线验收，真实QQ与完整生命周期待验收 |
| CHECK-021 | [原生凭据恢复与并发核对](credential-recovery.md) | 跨进程协调有界离线验收，底层根因未定 |
| CHECK-022 | [已停止实例的新运行代次](run-generation.md) | 元数据内核及停止后启动接合有界离线验收 |
| CHECK-023 | [同锁重启编排](restart-orchestration.md) | 有界离线验收 |
| CHECK-024 | [版本代次与回滚引用](version-generation.md) | 元数据有界离线验收 |
| CHECK-025 | [第二个固定 Framework 版本](second-framework-version.md) | 静态来源/配置核对及有界离线验收 |
| CHECK-026 | [已安装目标的更新编排](version-update-orchestration.md) | 已安装目标接合有界离线验收 |
| CHECK-027 | [停止前持久版本维护意图](durable-version-intent.md) | 有界离线验收 |
| CHECK-028 | [原动作绑定的回滚编排](rollback-orchestration.md) | 有界离线验收通过 |
| CHECK-029 | [真实启动输入修复与重试](live-start-repair.md) | 有界真实预登录及离线验收通过 |
| CHECK-030 | [CLI 真实私聊收发与历史对照](live-message-verification.md) | 有界真实验证通过，历史翻页有边界 |
| CHECK-031 | [历史包含锚点边界与媒体发送对照](history-boundary-media-send.md) | 有界修复与真实验证通过 |
| CHECK-032 | [消息读取功能性对照](functional-message-read.md) | 普通消息真实功能通过，转发样本待补 |
| CHECK-033 | [图片与文件下载功能性对照](functional-media-download.md) | 授权私聊合成附件功能通过 |
| CHECK-034 | [范围搜索功能性对照](functional-message-search.md) | 授权私聊正常功能通过 |
| CHECK-035 | [消息导出功能性对照](functional-message-export.md) | 授权私聊正常导出功能通过 |
| CHECK-036 | [合并转发窗口与API内容差异](forward-content-discrepancy.md) | 真实完整性验收失败，后端返回缺项 |
| CHECK-037 | [第二个合并转发样本](second-forward-sample.md) | 后端五节点展开及本轮窗口对照通过；旧缺项另见CHECK-036 |
| CHECK-038 | [指定群正常功能验收](functional-group-messaging.md) | 指定群文本/回复与读取功能通过 |
| CHECK-039 | [联系人与会话发现功能验收](functional-discovery.md) | 当前真实环境正常发现/详情通过 |
| CHECK-040 | [私聊回复、群提及及附件功能对照](functional-reply-mention-group-media.md) | API功能及本轮窗口人工对照通过，群文件定位缺陷已修复 |
| CHECK-041 | [实例预检与原生工作目录修复](lifecycle-working-directory.md) | 列表/在线复用通过，restart预检拒绝；CWD修复离线通过，真实后继未验收 |
| CHECK-042 | [本地启动实证与实例生命周期正常对照](local-startup-evidence-lifecycle.md) | 本地实证/安装恢复及重启/停止后启动通过，版本维护等仍待 |
| CHECK-043 | [版本维护功能验收](version-maintenance-live.md) | 固定版本切换/回滚正常流程通过 |
| CHECK-044 | [真实监听原生取消](event-native-cancellation-live.md) | 正常原生Ctrl+C功能通过 |
| CHECK-045 | [实例注销真实功能验收](instance-removal-live.md) | 正常停止后注销及重复查证通过 |
| CHECK-046 | [安装与卸载正常功能验收](installation-uninstall-live.md) | 固定包安装/缓存/引用保护/卸载/重新安装通过 |
| CHECK-047 | [双实验账号并行登录](multi-instance-login-live.md) | 两个不同账号正常登录/并行复用/选择通过 |
| CHECK-048 | [事件检查点、通知与有界恢复](event-recovery-live.md) | 有界恢复开发及正常功能验收通过 |
| CHECK-049 | [帮助、Schema与Skill对照](help-schema-skill-audit.md) | 当前公开面核对完成；严格字段及单一契约来源仍有差距 |
| CHECK-050 | [单源契约基础与安装管理验收](single-source-foundation-installation.md) | 切片通过 |
| CHECK-051 | [实例和诊断单源契约](single-source-instance-diagnostic.md) | 切片通过 |
| CHECK-052 | [资源发现契约整改](single-source-resource.md) | 切片通过 |
| CHECK-053 | [消息读取单源契约](single-source-message.md) | 切片通过 |
| CHECK-054 | [写操作及错误类型整改](single-source-write-error.md) | 切片通过 |
| CHECK-055 | [自省及响应桥清理](single-source-discovery-envelope.md) | 切片通过 |
| CHECK-056 | [事件契约整改](single-source-event.md) | 切片通过 |
| CHECK-057 | [输入契约单源整改](single-source-input.md) | 切片通过 |
| CHECK-058 | [单源契约整体验收](single-source-final.md) | 整改通过 |
| CHECK-059 | [附件与通知功能补验准备](attachment-notice-preflight.md) | 离线准备通过，真实补验待执行 |
| CHECK-060 | [附件格式与缓存清理正常功能验收](attachment-formats-live.md) | 附件正常功能通过，旁路事件错误后续修复见CHECK-062 |
| CHECK-061 | [群撤回与管理员变更通知验收](group-recall-admin-live.md) | 指定通知功能通过，两项观察后续已定位（CHECK-062、CHECK-063） |
| CHECK-062 | [无群通知哨兵导致事件解析失败的修复](groupless-notify-routing.md) | 已定位并修复，定向验收通过 |
| CHECK-063 | [成员角色旧值与上游刷新时序](member-role-cache-investigation.md) | 本地排查完成，已知上游缓存限制待跟踪 |
| CHECK-064 | [内部上下文迁移与分发边界](context-distribution-boundary.md) | 通过 |
| CHECK-065 | [Windows自动检查与Semifold持续交付验收](windows-automation-delivery.md) | 本地实现与验收通过，远端未运行 |
| CHECK-066 | [首次稳定发布与公开安装验收](first-stable-release.md) | 端到端通过 |
| CHECK-067 | [Windows 原生目录隔离与非 Windows 编译边界](platform-native-isolation.md) | 目录/编译切片已验证，完整平台验收未完成 |
| CHECK-068 | [NapCat macOS 固定发布资产复核](napcat-macos-release-audit.md) | 静态资产/源码核验，非运行验收 |
| CHECK-069 | [NapCat Windows 与 Apple Silicon 功能审计](napcat-macos-function-audit.md) | 静态审计完成，完整运行兼容性未验证 |
| CHECK-070 | [macOS Framework 手动预加载实验](macos-framework-manual-launch.md) | 真实启动尝试完成，Framework 预加载失败 |
| CHECK-071 | [macOS Framework 启动入口续查](macos-framework-entry-research.md) | 来源与加载链静态核对，候选未执行 |
| CHECK-072 | [Apple Silicon Framework 共享会话验证](macos-framework-session-validation.md) | 原型启动、共享身份及限定读取通过；非 CLI 平台交付验收 |
| CHECK-073 | [Apple Silicon 离线开发基线](macos-offline-baseline.md) | Offline verified；非运行支持验收 |
| CHECK-074 | [Apple Silicon 进程身份与退出观察](macos-process-identity.md) | Native synthetic verified；非运行所有权验收 |
| CHECK-075 | [Apple Silicon 已连接 TCP 归属与受检传输](macos-owned-connections.md) | Native synthetic verified；非运行生命周期验收 |
| CHECK-076 | [Apple Silicon 原进程代次定向终止](macos-exact-process-stop.md) | Native synthetic verified；非进程树停止验收 |
| CHECK-077 | [macOS 可执行文件候选发现](macos-executable-census.md) | 精确文件引用过滤、父退出/独立进程组及文件变更拒绝；不证明完整进程树停止 |
| CHECK-078 | [macOS 原代次暂停与恢复](macos-exact-process-suspension.md) | SIGSTOP/SIGCONT、SSTOP观察、暂停后重开及exec旧代次拒绝；非完整树冻结 |
| CHECK-079 | [macOS Keychain适配](macos-keychain-adapter.md) | 单程序合成读写/删除；跨程序清理拒绝与升级身份待办 |
| CHECK-080 | [macOS状态持久化](macos-state-durability.md) | 描述符相对发布、目录同步/设备flush及权限；非断电或完整目录安全验收 |
| CHECK-081 | [macOS原锁身份](macos-pinned-locks.md) | 状态/生命周期锁身份、POSIX属主/写权限、跨进程争用及事务发布复核 |
| CHECK-082 | [macOS启动效果锁重核](macos-dispatch-lock-guard.md) | 效果前失锁不派发、效果后失锁结果未知及原意图不重放 |
| CHECK-083 | [macOS签名服务恢复](macos-signed-service-recovery.md) | 原启动关联、平台/实例/身份校验及应用HTTP/WS接合；无生产记录创建 |
| CHECK-084 | [macOS原创适配核心](macos-framework-adapter-core.md) | 真实通知转交与原方法语义的14项合成测试；尚无生产入口部署 |
| CHECK-085 | [macOS入口编排](macos-framework-entry-orchestration.md) | 路径/宿主拒绝、Electron路径先行与原方法调用顺序；未部署启动 |
| CHECK-086 | [macOS启动准备](macos-launch-preparation.md) | Rust路径准备、全新环境及生成入口VM执行；未创建QQ进程 |
| CHECK-087 | [macOS入口资源发布](macos-adapter-bundle-publication.md) | 三文件整组改名、内容复核与失锁边界；未修改QQ副本 |
| CHECK-088 | [macOS副本入口配置](macos-copy-package-plan.md) | 摘要/版本及适配资源绑定的纯配置改写；未写入QQ副本 |
| CHECK-089 | [macOS应用静态审计](macos-qq-static-inspection.md) | arm64 Mach-O形状、六文件指纹及已安装QQ只读检查；非完整签名验收 |
| CHECK-090 | [macOS官方源签名](macos-original-signature.md) | 官方签名分类与ad-hoc拒绝；同步系统调用无硬取消能力 |
| CHECK-091 | [签名工作进程隔离](macos-signature-worker.md) | 截止、父存活、直接子进程回收及真实只读签名链路 |
| CHECK-092 | [macOS应用树基线](macos-bundle-inventory.md) | 全树普通内容/链接/模式清单与合成副本比对；不含xattr或部署 |
| CHECK-093 | [macOS扩展属性清单](macos-bundle-attributes.md) | 描述符属性摘要/资源叉/quarantine副本比较及已安装树只读核验；不含ACL或生产部署 |
| CHECK-094 | [macOS ACL与文件标志](macos-bundle-acl.md) | 描述符ACL摘要/st_flags比对及真实树只读核验；未完成权限加固或部署 |
| CHECK-095 | [macOS隐藏压缩属性](macos-bundle-compression.md) | 显示压缩属性、资源叉分块摘要及真实471项属性扫描；不含受控部署 |
| CHECK-096 | [macOS受控副本复制](macos-bundle-copy.md) | 私有暂存、描述符复制、完整比对与无覆盖发布；未完成最终加固/启动 |
| CHECK-097 | [macOS复制工作进程](macos-copy-worker.md) | 持久描述符通道、截止/父存活回收及整树隔离入口；不含最终加固/启动 |
| CHECK-098 | [macOS副本权限加固](macos-copy-hardening.md) | 活跃复制能力约束、ACL/模式收紧及内容保全；未完成持久恢复/部署 |
| CHECK-099 | [macOS副本入口部署](macos-package-deployment.md) | 活跃加固副本的package写入、适配资源重核及整树差异；未含签名/启动 |
| CHECK-100 | [macOS压缩入口部署](macos-package-compression.md) | 可写打开前复核、系统解压及非压缩元数据保全；仅合成验证 |
| CHECK-101 | [macOS副本签名权益](macos-signing-policy.md) | 固定角色权益与系统静态签名信息校验；未含生产签名器 |
| CHECK-102 | [macOS嵌套签名计划](macos-signing-plan.md) | 完整Mach-O覆盖、显式bundle角色匹配与内到外排序；不执行签名 |
| CHECK-103 | [macOS重签名载荷保全](macos-signing-payload.md) | 全部既有Mach-O slice非签名内容保全；不替代整树变化及权益核验 |
| CHECK-104 | [macOS签名整树变化](macos-signing-transition.md) | 精确映像/封印变化范围及资源、权限、链接保全；未接合生产签名器 |
| CHECK-105 | [macOS受控副本签名](macos-signing-execute.md) | 协作式签名内核、匿名FD权益、直接Child回收；生产监管/恢复待接合 |
| CHECK-106 | [macOS权益校验隔离](macos-policy-isolation.md) | 实际CLI固定角色校验、父存活/截止/guard取消；写入进程监管待接合 |
| CHECK-107 | [macOS签名写入监管](macos-signing-supervisor.md) | 目标/权益FD绑定、父死亡回收及完整合成事务；持久恢复待接合 |
| CHECK-108 | [macOS签名增量核验](macos-signing-incremental.md) | 原stamp摘要复用、当前目标变化范围及最终完整复核；总墙钟预算待验收 |
| CHECK-109 | [macOS签名结果同步](macos-signing-sync.md) | 文件/目录同步顺序、最终设备刷新及失败停止；不替代断电/持久恢复 |
| CHECK-110 | [macOS实例部署日志](macos-deployment-journal.md) | 原实例/输入绑定、一次执行、活跃签名凭据及只读恢复；真实装配/清理待接合 |
| CHECK-111 | [Framework 4.18.41固定资产](framework-4.18.41-catalog.md) | 原始ZIP通过生产解包器及arm64关键成员摘要；不代表正式启动完成 |
| CHECK-112 | [固定官网QQ来源](macos-fixed-source.md) | 固定完整树、载荷及Developer ID绑定；只读来源不授予复制或启动权 |
| CHECK-113 | [固定官网签名角色](macos-fixed-signing-roles.md) | 42个显式bundle角色、75个原生目标及PluginHost精确权益；未签署真实QQ |
| CHECK-114 | [部署效果前的入口计划](macos-prepublication-plan.md) | 无写入生成适配/改写计划、认证元数据读取，效果边界仍核验发布；完整装配待接合 |
| CHECK-115 | [固定输入与实例装配](macos-fixed-assembly.md) | 认证输入摘要、d2完整合成装配与不重放、真实资产无写入计划；CLI/真实副本待接合 |
| CHECK-116 | [受控运行目录凭据](macos-runtime-layout.md) | l2固定目录身份、准备失败/进程中断不重放及装配接合；CLI/真实部署待接合 |
| CHECK-117 | [macOS 首次凭据与配置准备](macos-runtime-material.md) | r2一次准备、原凭据恢复、固定配置及prepared提升；CLI/真实运行待接合 |
| CHECK-118 | [macOS 固定来源实例预约](macos-instance-reservation.md) | n2预约与共享登记协议、固定来源到r2及装配计划链路；CLI/真实启动待接合 |
| CHECK-119 | [macOS 固定镜像获取与缓存](macos-image-acquisition.md) | 固定字节与原身份、本地quarantine保留、同源HTTPS及完整暂存恢复；挂载/CLI待接合 |
| CHECK-120 | [macOS 只读挂载观察](macos-mount-observation.md) | 有界系统查询、唯一设备与实际只读根核验；挂载效果/恢复权威待接合 |
| CHECK-121 | [macOS 镜像挂载元数据转换](macos-image-mount-transition.md) | 原文件与全属性转换、有界校验缓存例外；挂载意图/监管仍待接合 |
| CHECK-122 | [macOS 磁盘附加身份](macos-media-generation.md) | 启动会话及整盘/卷对象ID、重附加同BSD名称拒旧；持久挂载权威待接合 |
| CHECK-123 | [macOS 一次挂载日志](macos-mount-journal.md) | m2一次意图、失败不重放、合成挂载只读恢复及卸载拒旧；生产监管/释放待接合 |
| CHECK-124 | [macOS 只读挂载监管](macos-attach-supervision.md) | 固定attach、一次许可/原FD、父死亡回收直接命令、日志争用中断；磁盘释放待接合 |
| CHECK-125 | [macOS 原磁盘FD释放内核](macos-device-pins.md) | IOMedia设备号与原FD绑定、busy拒绝、普通卸载后一次eject；持久释放待接合 |
| CHECK-126 | [macOS 持久挂载释放](macos-release-journal.md) | m2释放意图、双重消失证据、已消失恢复及跨进程不重放；效果监管/清理待接合 |
| CHECK-127 | [macOS 释放效果监管](macos-release-supervision.md) | 原raw FD跨进程传递、许可/存活监管及生产释放组合；清理/新请求接续待完成 |
| CHECK-128 | [macOS 挂载目录清理与后续请求](macos-mount-cleanup.md) | m2版本2原目录身份、空目录持久清理与新挂载接续；未知/旧记录保留 |
| CHECK-129 | [macOS 正式启动主线](macos-cli-startup.md) | start准备/一次sandbox派发/j2、原凭据恢复及CLI预算；真实QQ启动与完整生命周期待验收 |
| CHECK-130 | [macOS受管诊断](macos-managed-diagnostics.md) | 只读平台诊断及契约通过；正向兼容证据待接合 |
| CHECK-131 | [macOS启动软件证据绑定](macos-startup-evidence.md) | k2/v2输入及定向验证通过；真实正向链路待验收 |
| CHECK-132 | [macOS 原拥有运行停止与恢复](macos-owned-stop.md) | p2/原audit代次/继承范围与CLI停止、成员收敛及合成恢复；真实QQ生命周期仍待验收 |
| CHECK-133 | [macOS 正式 CLI 基础读取与生命周期验收](macos-cli-basic-acceptance.md) | 正式 GUI/登录、限定读取、k2/doctor、停止及再次登录后的重启通过；非完整平台验收 |
| CHECK-134 | [macOS 同实例 QQ 数据继承与登录恢复](macos-profile-inheritance.md) | 原目录身份保持、保留账号点击登录、同账号限定读取及诊断通过；中间退出由用户手动操作 |
| CHECK-135 | [macOS CLI 跨构建 Keychain 授权恢复](macos-keychain-upgrade-recovery.md) | 系统三项授权在有效时限内完成；新旧程序同账号读取、原状态保持及诊断通过，无需Developer ID证书 |
| CHECK-136 | [macOS 正式 CLI 功能与分发集中验收](macos-cli-functional-acceptance.md) | 固定组合收发/媒体/历史/事件、同实例附件继承、两版切换回滚及本地安装升级通过；扩展矩阵和公开交付另列 |
| CHECK-137 | [双平台原生质量门禁修复](native-quality-ci.md) | Windows/macOS 原生离线门禁、release构建及平台归档通过；非真实QQ扩展矩阵或线上发布 |

## 产物与公开来源索引

以下保留既有来源编号，便于读取早期证据。来源表中的历史候选/状态仅适用于当时检索，
main/latest 和网页均为可变来源，不作为新的固定基线；固定提交、检索日期和摘要以关联 FACT/CHECK 为准。

| 来源编号 | 来源/标识 | 当时用途与限制 |
|---|---|---|
| SRC-001 | https://github.com/NapNeko/NapCatQQ/blob/main/packages/napcat-shell-loader/launcher.bat | Windows Shell 启动参数 |
| SRC-002 | https://github.com/NapNeko/NapCat-Windows-Boot/blob/main/main/main.cpp | 创建新进程和加载 DLL 的公开实现 |
| SRC-003 | https://github.com/NapNeko/NapCatQQ/blob/main/packages/napcat-shell-loader/qqnt.json | Shell 入口配置 |
| SRC-004 | https://github.com/NapNeko/NapCatQQ/blob/main/packages/napcat-shell/base.ts | Shell 引擎、登录和会话初始化 |
| SRC-005 | https://github.com/NapNeko/NapCatQQ/blob/main/packages/napcat-framework/napcat.cjs | 桌面会话代理 |
| SRC-006 | https://github.com/NapNeko/NapCatQQ/blob/main/packages/napcat-framework/nativeLoader.cjs | 注入初始化对象的函数入口 |
| SRC-007 | https://github.com/NapNeko/NapCatQQ/blob/main/packages/napcat-core/apis/msg.ts | 内部消息服务调用 |
| SRC-008 | https://github.com/NapNeko/NapCat-Windows-Boot/blob/main/hook/hook.cpp | 启动读取重定向和 Hook |
| SRC-009 | https://napneko.github.io/guide/boot/Framework | 官方现有 LiteLoader 安装途径 |
| PIN-001 | https://github.com/NapNeko/NapCatQQ/releases/tag/v4.18.38 | 发布标签和附件 |
| PIN-002 | https://api.github.com/repos/NapNeko/NapCatQQ/releases/410166712 | release/asset ID、长度、平台 digest |
| PIN-003 | https://github.com/NapNeko/NapCatQQ/blob/bbd0d2aca3d470c20b33b1d4f95123335633bf26/LICENSE | 自定义根许可 |
| PIN-004 | https://github.com/NapNeko/NapCatQQ/blob/bbd0d2aca3d470c20b33b1d4f95123335633bf26/README.md | 独立项目授权与混合许可限制 |
| PIN-005 | https://github.com/NapNeko/NapCatQQ/tree/bbd0d2aca3d470c20b33b1d4f95123335633bf26/packages/napcat-napi-loader | 原生二进制、启动 BAT；目录无可重建实现 |
| PIN-006 | https://github.com/NapNeko/NapCatQQ/blob/bbd0d2aca3d470c20b33b1d4f95123335633bf26/packages/napcat-framework/nativeLoader.cjs | 对象接入和异常处理 |
| PIN-007 | https://github.com/NapNeko/NapCatQQ/blob/bbd0d2aca3d470c20b33b1d4f95123335633bf26/packages/napcat-framework/napcat.ts | 初始化和生命周期 |
| PIN-008 | https://github.com/NapNeko/NapCatQQ/blob/bbd0d2aca3d470c20b33b1d4f95123335633bf26/packages/napcat-framework/vite.config.ts | 产物装配和依赖 |
| TRACE-001 | https://github.com/NapNeko/NapCatQQ/commit/080e4d198fb331fc05e0c5ad79ccd7bf42e451e6 | 引入预编译原生文件和脚本；本轮通过 Git 对象核对 |
| TRACE-002 | https://github.com/NapNeko/NapCatQQ/commit/ed19c52f257d19b021ee71f4d66a49eaf7d59dcc | 目录迁移，旧/新 blob 一致 |
| TRACE-003 | https://github.com/NapNeko/NapCatQQ/commit/9109b8cfe2f46bc6f4f80cdeec8d1621d06e186d | BAT/CJS 路径修复，非目标二进制更新 |
| TRACE-004 | https://github.com/NapNeko/NapCatQQ/commit/ec6aadaeea38ed3bca67b82bab3d7a9c0f91b8d8 | QQ 9.9.36 Shell 修复，不能作为 Framework 支持依据 |
| TRACE-005 | https://github.com/NapNeko/NapCatQQ/blob/bbd0d2aca3d470c20b33b1d4f95123335633bf26/.github/workflows/release-publish.yml | 原生文件复制打包及 Shell 专属验证边界 |
| TRACE-006 | https://github.com/NapNeko/NapCat-Native/tree/9df503de54401b4484e60d8090c6dd05cb94775a | 相邻 node-gyp addon，未建立与目标 PE 的映射 |
| REPORT-001 | https://github.com/shuakami/qq-chat-exporter/issues/485 | 下游 QQ 9.9.26-44343 的用户报告，不是本项目或原版同会话验收 |
| REPORT-002 | https://github.com/shuakami/qq-chat-exporter/issues/332 | 界面/接收线索，未给明确 QQ 和原生摘要 |
| REPORT-003 | https://raw.githubusercontent.com/shuakami/qq-chat-exporter/master/scripts/build-framework-plugin.py | 2026-10-16 读取的可变脚本，显示下游重写 BAT；不当作历史包内容证明 |
| HISTORY-001 | https://github.com/NapNeko/NapCatDocs/commit/c13e2b9bed8add13d816c69049affaf425b06377 | 2025-05-12 增加 LiteLoader 维护警告 |
| HISTORY-002 | https://github.com/NapNeko/NapCatDocs/commit/d1c261af78d53e57bd594ed483392a5a3bf08ace | 删除旧 Installer，不是 napiLoader |
| HISTORY-003 | https://github.com/NapNeko/NapCatDocs/commit/5f07655fc59b24f58b8adad79699d998a0a34619 | 简化指南并保留 LiteLoader；无原生缺席解释 |
| HISTORY-004 | https://github.com/NapNeko/NapCatQQ/issues/1375 | 2025-11-10 Member 回复弃用 Once/OneKey，推荐原生 BAT |
| HISTORY-005 | https://github.com/NapNeko/NapCatQQ/issues/1398 | 2025-12-05 Member 回复原生入口可有头加载 |
| SAFETY-001 | https://github.com/LiteLoaderQQNT/LiteLoaderQQNT | 归档及账号风险警告，2026-10-16 读取的可变页面 |
| SAFETY-002 | https://napneko.github.io/other/security | 官方讨论账号影响/风控，不是免封声明；历史配置不直接移用 |
| SAFETY-003 | https://github.com/NapNeko/NapCatQQ/discussions/517 | 历史 Hook/反检测说明，不代表当前效果 |
| SAFETY-004 | https://github.com/NapNeko/NapCatQQ/commit/62c92463681365feeffe634ec7d55eb3dbcf5f50 | 提交主题说明反检测意图 |
| SAFETY-005 | https://github.com/NapNeko/NapCatQQ/commit/5c72f771c37273c5c3e91a655b777ae991818a7f | 说明两种模式初始化调用 enableAllBypasses |
| SAFETY-006 | https://github.com/NapNeko/NapCatQQ/commit/b9f61cc0eee02465e9a69b3d99675c304265af35 | 配置反检测及 Bypass 崩溃恢复的提交说明 |
| SAFETY-007 | https://github.com/NapNeko/NapCatQQ/blob/bbd0d2aca3d470c20b33b1d4f95123335633bf26/packages/napcat-shell/base.ts | 固定 Shell 调用点；Framework 见 PIN-007 |
| SAFETY-008 | https://github.com/NapNeko/NapCatQQ/blob/bbd0d2aca3d470c20b33b1d4f95123335633bf26/packages/napcat-core/packet/handler/napi2nativeLoader.ts | 原生导出类型和加载逻辑，不是原生实现 |
| SAFETY-009 | https://github.com/NapNeko/NapCatQQ/blob/bbd0d2aca3d470c20b33b1d4f95123335633bf26/packages/napcat-core/helper/config.ts | Bypass 配置字段，不证明所有平台默认效果 |
| CLI-001 | https://github.com/NapNeko/NapCatQQ/blob/bbd0d2aca3d470c20b33b1d4f95123335633bf26/packages/napcat-onebot/action/go-cqhttp/GetFriendMsgHistory.ts | 私聊历史参数、取最近消息及构造消息 ID |
| CLI-002 | https://github.com/NapNeko/NapCatQQ/blob/bbd0d2aca3d470c20b33b1d4f95123335633bf26/packages/napcat-onebot/action/msg/GetMsg.ts | 消息 ID 映射依赖，不承诺跨重启使用 |
| CLI-003 | https://github.com/NapNeko/NapCatQQ/blob/bbd0d2aca3d470c20b33b1d4f95123335633bf26/packages/napcat-onebot/action/msg/SendPrivateMsg.ts | 私聊发送入口，不证明上游幂等或对方收悉 |
| CLI-004 | https://github.com/NapNeko/NapCatQQ/blob/bbd0d2aca3d470c20b33b1d4f95123335633bf26/packages/napcat-onebot/action/system/GetStatus.ts | online 及固定 good=true，非全面健康检测 |
| CLI-005 | https://github.com/NapNeko/NapCatQQ/blob/bbd0d2aca3d470c20b33b1d4f95123335633bf26/packages/napcat-onebot/action/system/GetVersionInfo.ts | NapCat app_version 与 OneBot protocol_version |
| CLI-006 | https://github.com/NapNeko/NapCatQQ/blob/bbd0d2aca3d470c20b33b1d4f95123335633bf26/packages/napcat-onebot/action/system/GetLoginInfo.ts | 当前账号信息，不自动选定目标账号 |
| TOOL-001 | https://nodejs.org/en/about/previous-releases | 历史 Node 候选依据，已被用户 Rust 决定替换；未安装工具链 |
| TOOL-002 | https://nodejs.org/docs/latest-v24.x/api/test.html | 历史 Node 候选依据，不再作为当前测试路线 |
| TOOL-003 | https://doc.rust-lang.org/cargo/reference/manifest.html | Cargo manifest、edition 与 rust-version；工程验证另见 CHECK-008 |
| TOOL-004 | https://docs.rs/clap/latest/clap/ | Rust 命令解析候选，2026-10-17 可变 latest 文档 |
| TOOL-005 | https://docs.rs/reqwest/latest/reqwest/blocking/struct.ClientBuilder.html | blocking 客户端代理/重定向/重试及超时配置；不是组合构建通过证据 |
| AGENT-001 | https://github.com/larksuite/cli | 官方 README 的 schema、JSON、分页、dry-run 和 Agent 指引；不复制实现或特殊命令前缀 |
| AGENT-002 | https://opencode.ai/docs/skills/ | SKILL.md 按需发现/加载；本工具导出及实际宿主验收尚未实现 |
| UX-001 | https://github.com/NapNeko/NapCatQQ/releases/tag/v4.18.38 | 指定 release/tag 与提交；网页不代替已登记产物摘要，也未给完整原版兼容矩阵 |
| UX-002 | https://napneko.github.io/guide/boot/Framework | 现行指南要求确认加载框架支持 QQ，主要描述 LiteLoader，不能直接当作本方案原生入口支持证明 |
| UX-003 | https://napneko.github.io/guide/boot/Shell | Shell 指南与兼容信息，启动模式不同，不移用为 Framework 原生加载兼容保证 |

附属原始审计索引：[NapCat v4.18.38](napcat-v4.18.38.json)、[原生来源](native-loader-provenance.json)、
[macOS 旧基线资产](napcat-macos-assets.json)、[旧基线 Action 索引](napcat-macos-actions.csv)。
这些结构化索引保留原固定版本范围；v4.18.41 原型以 CHECK-072 为准。
