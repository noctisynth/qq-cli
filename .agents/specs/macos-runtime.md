# Apple Silicon Framework 运行适配方案

文档编号：SPEC-008。状态：Active，正式 CLI 平台技术契约。更新：2026-10-27。
需求：RFC-0008。事实：FACT-005；验收与剩余范围：TASK-006；历史原型：CHECK-072，正式 CLI：CHECK-136。

## 装配与签名

以固定官方 DMG 建立工具拥有的专用 QQ 副本，保留原安装；不复制真实 QQ 私人数据。
副本 package.json.main 指向外部薄适配入口，继续原 QQ application.asar/app_launcher/index.js。
只对专用副本本地 ad-hoc 重签名。已验证的原生 hook 需要 allow-jit、disable-library-validation，
主进程另需 allow-unsigned-executable-memory；不加入 disable-executable-page-protection，不关闭 Gatekeeper/SIP。
这是验证组合的配置，不是其它 QQ/系统版本的通用白名单；生产安装需重复验证来源、完整性和权限范围。

移除副本的 App Sandbox/共享组权益后，正式 CLI 使用下述外部 sandbox 强制受控目录与执行范围，并以签名记录、进程及连接身份约束所有权。原型脚本不作为产品安装入口；固定组合的正常验收与扩展故障范围见 TASK-006。

## 固定官网应用来源

install/macos::source::Source仅接受官网7.0.2-53644基线：固定DMG URL/长度/SHA-256为获取层提供描述，不把它们当作已完成下载或挂载的凭据。Source先对规范app根建立完整Inventory v4并匹配固定原树指纹，再通过qq::macos::inspect_original隔离检查腾讯Developer ID签名，匹配固定版本/构建、原生arm64及选定载荷指纹，最后重读完整清单并检查原节点身份。App Store来源、同版本不同载荷或元数据均拒绝，不为匹配指纹自动规范化原树。
Source私有构造，仅暴露只读Application与Inventory及完整重核；没有owner、签名完成或启动能力。复制/部署在效果边界重核源及生命周期guard。原树指纹包含内容、链接、模式、flags、ACL和属性，不能把本地实验副本指纹替换为官方基线。下载/只读挂载监管、角色清单及完整部署输入摘要由下述独立模块接合；失败现场清理的差距见TASK-006；本观察不证明所有签名组件运行兼容。

## 副本签名权益策略

macos::signing::Policy生成完整的新权益模板，不合并原签名权益：Main仅allow-jit、disable-library-validation、allow-unsigned-executable-memory三个布尔true；Electron与PluginHost仅前两个；Native不含权益。Main仅适用于已审查的QQ主进程，Electron须由安装布局明确指认宿主/helper，不能因文件位于.app就自动赋予。禁止保留App Sandbox、共享组、get-task-allow、disable-executable-page-protection及其他额外权益。PluginHost只用于固定官网清单中的截图插件宿主，保留其原有的这两项动态代码权益，不因此把它认作Electron，也不授予unsigned-executable-memory。具体映射见下节；组件运行兼容性仍须独立验证。
Policy.verify是同步只读观察，路径须规范绝对且无链接，前后复核所选节点身份/元数据。使用严格资源、嵌套代码、所有架构和禁止网络的Security Framework校验，再以SecCodeCopySigningInformation读取系统识别的权益字典和静态flags；不解析codesign人类输出，不把XML导出缺失当无权益。只接受CS_ADHOC|CS_RUNTIME（0x10002），权益字典数量和每个固定key的CFBoolean true精确匹配；Native允许空字典；字典缺省时还要求原始权益blob缺省，SDK的“非字典格式”不能被当作无权益。所有架构的签名有效性不意味着外来架构的权益集合已逐一审计；精确权益验证限本机原生arm64。
FFI限定native::macos_signing_info：SDK成功返回的CFDictionary遵循Create规则释放；静态key按Get规则保留，flags/字典/布尔值分别运行时类型检查。不调用签名私钥、Keychain密码接口或QQ，不把本地ad-hoc身份解释为腾讯来源。调用前后截止属于协作式限制；阻塞OS调用的隔离、嵌套角色顺序、受控副本完整清单、签名执行及失败恢复未由该观察完成。

## 嵌套签名目标计划

Inventory.signing_plan只读建立绑定原根路径及完整指纹的计划，不授予复制/签名权威。输入为固定版本审查所得的显式BundleRole：相对bundle路径、CFBundleIdentifier、CFBundleExecutable和Policy；仅根可为Main且根必须为Main。路径拒绝绝对/父级跳转/控制字符，标识和执行文件名单独限定长度与字符，最多256个bundle。调用方仍须认证来源及固定版本角色清单；由应用自身字段自填清单不能证明权限角色已审查。
从完整Inventory逐文件以保留目录描述符读取头部，复核原stamp；识别Mach-O magic后复用有界thin/fat32/fat64 arm64解析，不依据扩展名或执行位遗漏native载荷。只接受MH_EXECUTE/MH_DYLIB/MH_BUNDLE；不增加Intel/Rosetta支持。枚举全部实际.app/.framework/.xpc/.appex/.bundle目录（含根），要求与输入一一对应，未知/重复/缺失bundle拒绝，不自动授予Electron例外。
每个bundle要求1MiB以内的当前布局Info.plist（应用Contents、Framework的Resources），规范解析仍位于本bundle内且属于原清单普通文件。Info.plist通过原目录/文件描述符有界读取并核对原SHA-256、长度及stamp，再以系统CFPropertyList解析标识/执行名；不依赖路径重读或人类文本。SecCodeCopySigningInformation通用元数据提供签名系统识别的主可执行文件，若同时提供secured Info.plist则额外核对两种解析的身份字段一致；该字段是可选值，缺失不当作无Info，也不把通用元数据读取当签名有效性校验。所指主可执行文件须规范解析至本bundle内、属于已检查的Mach-O清单、文件名/标识匹配审查值，且不能被多个bundle重复认领。应用/XPC/app extension要求MH_EXECUTE，Framework要求MH_DYLIB，其余code bundle要求MH_DYLIB或MH_BUNDLE；Framework和其他code bundle不授予Electron权益。
每个native映像恰好对应一个目标：bundle主程序通过bundle根签署，剩余普通native文件作为Native目标；Framework/Current等链接不重复签署。按目标路径深度从内向外稳定排序，最终根应用最后，计划保留bundle标识、角色和真实主文件相对路径。创建前后重核整树；revalidate要求同根和同指纹并重新扫描。目录/文件/属性变化或新清单替换均不能复用旧计划。
此处支持既定应用布局；没有有效Info/主可执行文件或只有资源的bundle、未知Mach-O类型/架构不静默忽略。计划是后续受控签名器的输入，不自动执行codesign --deep，不证明真实角色分配、运行兼容、嵌套签名已完成或持久恢复成立。系统元数据调用及全树读取仍采用协作式截止；生产工作进程隔离与执行效果边界由下述受控签名器组合。

## 固定官网版本角色清单

macos/source/roles.rs显式列出已认证官网7.0.2-53644的43个bundle路径、标识、执行名与角色；不从目录名、原权益或字符串动态推断权限。主QQ为Main；主QQ的4个helper与QQEXDOC/QQEXDesktopPet/QQEXGuild/QQEXMiniProgram各自宿主及4个helper共24项为Electron；QQ ScreenCapture plugin为PluginHost；17个框架/登录、原生helper及分享扩展share.appex为Native。其余独立Mach-O仍由完整清单枚举为Native。PluginHost与Electron生成相同的两项权益，但有独立私有worker角色名plugin-host，不能取得Main第三项权益。
Source.signing_roles提供固定元数据供已认证副本编辑后的计划使用，不认证任意副本；Source.signing_plan仅针对原树创建只读计划，完整重核并要求所有实际bundle及原生映像被精确覆盖。原树计划不能直接用于不同根或已改入口的副本；副本须在自身Inventory上重新建立计划，再由活跃复制能力执行。此清单是固定版本签名配置，不等于这些组件的GUI/媒体/长期运行已验收。

签名计划显式识别.appex并将其可执行文件和精确封印归入同一目标，缺少固定角色拒绝；不得将其作为独立Mach-O签署后忽略封印变化。

## 重签名载荷保全

签名计划为每个Mach-O保存独立的qq-cli-macos-resign-payload-v1摘要，不改变Inventory v4。解析共享thin/fat布局并覆盖全部arm64/x86_64 slice；保留既有外来slice不意味着支持Intel运行。绑定容器格式、slice顺序、CPU/subtype及alignment，允许重签名导致fat偏移/长度重排，但表外与slice间填充必须全零。
每slice必须有唯一LC_CODE_SIGNATURE及无section的只读__LINKEDIT；签名分配区位于文件末尾，LINKEDIT结束于slice末尾且不与其他segment文件/VM范围重叠。签名分配区限12字节至64MiB，SuperBlob头和声明长度有界；LINKEDIT虚拟大小受实际文件大小和页对齐约束。摘要仅排除完整签名分配区，并规范化LC_CODE_SIGNATURE.datasize、__LINKEDIT.filesize/vmsize三个字段，签名起点及其他命令、代码和数据全部保留。系统codesign可能保留SuperBlob声明长度之后的非零签名分配填充，因此不能要求该分配区填充全零；OS签名有效性仍须独立验证。
SigningPlan.verify_payloads在同根新Inventory上经持有目录描述符逐一比较原目标摘要，前后重核清单。它仅证明既有映像载荷保全，不接受资源/属性变化、不证明没有新增文件、不核验新权益，也不返回部署凭据；后续签名事务必须另行验证允许的整树变化和Policy。普通revalidate仍要求原指纹，不能用于接受已签名新树。解析全文件限2GiB、load commands限4096/16MiB，流式读取检查截止；系统文件调用不具有绝对墙钟保证。

## 已封印副本的签名变化核验

SigningPlan.verify_transition将原计划绑定的Inventory快照与同根签名后清单比较。当前仅接受原有完整封印布局：每个bundle的CodeResources必须已是原清单普通文件；应用取Contents/_CodeSignature/CodeResources，Framework取经过原元数据核验的实际版本目录下_CodeSignature/CodeResources。不以任意同名目录作为例外，缺失原封印不自动新增或补建。
整树路径集合必须相同。仅计划原生映像与上述精确封印文件允许内容、长度、inode和时间变化，仍须保留设备、属主/组、链接数、模式、ACL、文件标志及全部扩展属性。系统codesign可能替换可执行文件，故不能要求其inode不变；这些可变文件的直接父目录只允许时间变化，目录inode/大小/权限等保持；所有其他节点内容、元数据与stamp完全一致。随后执行全部载荷摘要保全及新树前后重核。
此核验不消费或产生写能力，不验证签名有效性/权益，也不返回安装凭据。生产事务仍须在活跃受控副本上执行、分别核验全部Policy并持久化失败状态。尚未支持新增封印布局或签名导致的压缩/属性规范化，观察到这些变化时拒绝而非默许；固定版本部署由下述assembly和d2事务组合，验收范围见TASK-006。

## 受控副本的协作式签名内核

Inventory.sign_copy消费活跃复制能力，拒绝普通扫描和非加固树。先核对原私有父目录、调用方guard、完整计划和已封印布局；要求当前UID、无ACL及0700目录/0600加原所有者执行位的文件。仅按计划从内到外调用固定/usr/bin/codesign，固定ad-hoc、无时间戳、runtime、DER和完整角色权益，清空环境、工作目录为根、丢弃工具输出，不使用shell、--deep或任意工具参数。
权益通过原父目录FD独占创建0600普通文件，先unlink再写固定模板并seek至起点；传入stdin作为普通文件描述符，以/dev/fd/0读取。文件必须属于当前UID、零链接、无ACL；不把管道或符号“-”作为可用的权益输入。没有可再次按临时路径打开的权益文件；中断前尚未unlink的空文件或unlink失败不自动认领/删除。
每次签名前重核guard和当前整树stamp；等待直接Child时持续检查截止和guard。失败/截止仅对仍未回收的原Child尝试一次终止，最多另加2秒确认回收；wait查询不确定后不发信号，未知退出返回SIGNING_CLEANUP_UNKNOWN。失败消费能力并保留可能部分签名的副本，不自动回滚/重试。目标成功退出后执行增量清单核验（规则见下节），比较本步骤允许变化和载荷保全，验证该目标Policy；全部目标后再次核验全部Policy、整树及guard才返还活跃副本能力。
同步sign_copy是协作式内核与合成对照，不作为生产启动入口；生产使用下述sign_copy_supervised组合隔离Policy校验、codesign监管、结果同步及d2持久记录。逐目标增量核验见下节。父端阻塞文件系统的绝对墙钟预算仍有限制，返回Inventory本身不是持久安装回执。

## 签名权益校验的工作进程隔离

Policy.verify_isolated使用当前CLI可执行文件的私有--internal-macos-policy-v1入口，清空环境、丢弃输出，仅传规范路径、固定main/electron/plugin-host/native角色和1至300000毫秒预算。worker仅调用同步Policy.verify，无签名、凭据、安装状态或QQ执行能力。main在公开参数解析前分派；没有注册入口的库宿主失败，不回退同步校验。
来源校验与权益校验共享native层只读工作进程管理，native不依赖napcat的Policy。父进程在启动前、等待期间及接收结果后核对截止/调用方guard，只接受权益专用95退出码；来源91/92不混用。worker具有独立截止线程和stdin存活管道观察，父管道EOF或意外数据使其退出；线程可终止阻塞的系统验证。失锁/截止关闭管道，只对仍未回收的直接Child尝试一次终止，另加最多2秒确认；wait结果不确定不发信号，无法回收返回SIGNATURE_WORKER_CLEANUP_UNKNOWN，Drop不重复操作已消费Child。
Inventory.sign_copy_supervised将逐目标及最终全部Policy验证接入该隔离路径，仍保留父端整树/载荷及活跃副本守卫。同步sign_copy用于明确的协作式内核与合成对照。该只读工作进程仅隔离系统验证；codesign写入监管见下节。逐目标重扫成本和结果落盘由下述增量核验及同步处理；父端文件系统预算和失败恢复限制见TASK-006，只读校验结果本身不是启动凭据。

## 签名写入进程监管

Inventory.sign_copy_supervised在逐目标签名时启动当前宿主的独立监管入口，接合隔离Policy验证；替代仅隔离校验的早期入口。父端仍消费活跃加固副本，核对原目录、guard与整树stamp。向监管者传递原目标描述符、零链接权益普通文件和stdin存活管道，参数只有目标规范路径、固定角色、1至1800000毫秒预算；不传替代工具或任意权益。监管者不向继承的stdout/stderr写入：它们分别承载权益FD和目标FD；复核权益原字节/UID/0600/零链接/无ACL，以及目标路径与FD的完整stamp、当前UID/私有模式/无ACL后，才创建固定codesign Child。
监管者自己的线程观察父管道，EOF或数据使存活标记失效；主循环检查标记和截止，通过原Child执行一次终止并最多2秒确认回收。父端失锁/截止/Drop关闭存活管道，额外最多3秒等待监管者报告；不能先杀监管者而遗留codesign。只有109成功、110已结束失败、111已结束截止可表示监管流程已结束；112、信号退出、未知退出码或等待错误均为SIGNING_CLEANUP_UNKNOWN，不授予成功或重试权限。监管者返回成功后，父端仍须核对截止/guard、整树载荷和各角色Policy。
该机制覆盖父CLI突然死亡后的管道关闭，不承诺监管者自身被外部强制杀死、内核不可回收进程或不可中断spawn/文件调用时的绝对完成。此时保留未知结果和副本，后续持久恢复必须独立核验。codesign只签名指定目标，不启动被签应用。成功结果按下述同步及d2协议持久化；总读取预算与失败现场恢复仍有限制，返回内存能力不等于持久安装回执。

## 签名期间的增量清单核验

每个签名步骤之后，inspect_using以刚刚验证的内存清单作为私有缓存，仍逐层保留目录FD、枚举全部节点、打开每个节点并比较完整stamp，最后再次核对整树stamp。只有同根且设备/inode/大小/模式/UID/GID/链接数/标志/mtime/ctime全相同的节点，才复用内容、属性和ACL摘要；其它节点完整重读。缓存不序列化、不授予owner能力，不用于公开inspect/revalidate。缓存文件大小与属性（含资源叉）仍计入原总预算，不能靠复用绕过限制。
逐目标verify_step只允许本目标的真实可执行文件、精确CodeResources及它们直接父目录的规定变化；其它目标的映像/封印也必须不变。复核当前目标全部既有slice的原始载荷摘要，并再次核对stamp后才进入下个目标签名。最终verify_transition及Policy检查保留，整树内容重新读取，不使用缓存替代最终验收。
这消除了逐目标反复读取全部未变化内容及其它目标载荷的应用层成本；仍每步枚举/检查全树元数据，Security Framework也可能独立读取嵌套代码。不得将局部读取量测试扩展为总墙钟、远程文件系统或大应用完整预算已验收。父端阻塞文件系统隔离与失败现场恢复见TASK-006；成功结果按下述同步及d2协议持久化。

## 签名结果同步

完成全部签名、权益及整树变化核验后，签名事务通过当前清单的原目录链打开每个计划映像和精确CodeResources，去重后执行F_FULLFSYNC；再对这些文件的直接父目录按深度由内到外执行fsync，随后同步活跃复制能力保留的私有父目录（包含匿名权益文件的创建/取消链接）。最后再次对一个已验证的签名普通文件执行F_FULLFSYNC，使目录同步之后也有设备刷新请求。清单已约束整棵app同设备，复制父目录身份持续由owner守卫核对。
每个同步调用前后核对截止、guard和全树stamp；通过signing_file逐层持有描述符并核对当前节点身份，不按未经验证的路径同步，也不重新打开替代文件。任何调用失败、路径变化、失锁或截止立即失败，消费能力并保留部分现场，不继续后续同步、不自动重试或返回成功Inventory。同步完成后仍执行最终guard/stamp复核。
这是操作顺序和系统返回值验证，不是断电实验、跨文件系统事务或持久安装日志。部署意图/完成回执必须独立落盘，恢复不能从扫描到已签名副本推导写入授权。同步系统调用仍为父端协作式截止，阻塞文件系统隔离与失败现场恢复见TASK-006；持久部署记录由下述d2协议负责。

## 实例部署意图与只读恢复

runtime/macos/deploy使用独立macos-deployments索引与d2认证记录，按runtime_store同一(instance_id,run_id)摘要定位；目标路径固定为该运行目录下QQ.app，不接受记录中的任意路径。Target绑定实例、运行代次、NapCat安装ID/摘要和完整部署输入摘要，request_id记录本次意图；完整来源、固定版本、适配资源、角色清单等输入的认证及摘要构造由下述assembly::Plan负责，不由一个格式合法的hash自动证明。d2不读取/迁移Windows运行或安装记录。
新部署要求注册表中实例/安装唯一且绑定一致，实例为prepared、无账号/进程标记，目标app不存在。先持有原生命周期锁，再于状态事务内发布Pending认证记录，落盘后才调用一次操作。长操作不持有状态锁；guard每次用短事务复核原生命周期锁、最初状态锁的目录/文件身份、完整实例/安装快照及原Pending记录。原锁被替换不接受新锁继续。相同代次只能有一条意图，不同请求键/输入冲突，不能绕过旧Pending或Unknown。
操作回调必须在实际效果边界使用guard；只有成功执行完整签名/同步且仍拥有复制能力的Inventory可消费为Artifact。普通inspect、复制或仅加固不能生成完成凭据。Artifact记录整树摘要、app及私有父目录设备/inode和UID；消费前重核活跃能力、guard和全树。成功在原锁/快照下发布Complete；操作失败发布Unknown，若失锁/截止或保存失败无法完成发布，保留Pending并返回结果未知，禁止自动重试。registry的实例状态不由部署日志改成ready，日志不授予启动/账号/进程权限。
恢复将Pending视为Unknown，不重新执行。Complete恢复重新认证d2记录、检查当前注册表作用域和输入绑定，派生固定app路径，复核原父目录身份/私有权限/无ACL、原app身份、所有节点UID和完整内容指纹；只返回观察结果，不恢复owner写能力。已完成部署重试也执行这些检查而不再调用操作。原签名密钥缺失时，即使references文件丢失，macos-deployments残留仍禁止创建替代密钥。
正式启动通过assembly::Plan生产完整输入并调用d2；失败现场清理/写权限恢复与断电验收不由只读完成恢复提供，差距见TASK-006。模拟操作失败或进程退出不是电源中断证明，也不保证所有文件系统上的绝对持久性。

## 真实会话转交

适配在官方入口前安装精确 wrapper.node 注册观察，仅处理副本的固定模块路径，其他模块原样转发。
原 QQ 自己调用 getNTWrapperSession("nt_1") 时，向原版 nativeLoader.cjs 的 initializeNapCat 转交取得的原生对象和登录服务。
不调用不存在的 session.create，不创建另一 session，不运行 Shell 初始化。

原生 init/getter 属性不可写；通过可替换的模块导出和转发视图接入，保留原 receiver、参数、返回、异常和其它方法。
观察官方 init 的真实 onOpentelemetryInit，先执行原回调，再在 is_init === true 且代次匹配时释放 Framework 初始回调。
先注册/后通知或先通知/后注册都必须正确；拒绝过期代次及 truthy 非布尔成功，初始就绪只释放一次。
该初始化观察不承担切号/重连状态机；账号变化及连接恢复仍由各自用例核对，真实扩展范围见TASK-006。

当原生 getNTUserDataInfoConfig 返回空值时，外部适配回退到受控 userData；非空原生结果原样保留。
每次精确模块注册均应用该兼容，不改上游文件；生产入口检查最终数据路径属于本实例的认证布局。

原创适配核心维护于 napcat/runtime/macos/adapter/bridge.cjs，由 Rust include_str 提供给装配器；加载文件本身不安装钩子。安装函数只观察给定绝对 wrapper 路径，恢复时不覆盖后来安装的观察器。entry.cjs提供显式调用的入口编排：验证规范绝对路径、无别名和单链接文件，实例与Framework根分离；仅接受专用QQ.app的darwin/arm64进程，要求Electron尚未ready。目录必须由生命周期预先建立，适配不自动创建。先设置并回读全部Electron路径，再加载原版nativeLoader、安装观察并调用原QQ入口；同步异常保留并恢复观察器，异步Framework失败只报告固定错误码。
该路径检查不认证安装来源/签名/摘要，也不封闭检查后的替换竞争；这些边界由下述部署、启动及失败日志组合约束，启动器main兼容按下一段处理。Node合成测试不替代固定QQ/Framework组合回归。

原QQ入口返回后，适配器通过setImmediate等待启动器初始化，再读取globalThis.launcher.installPathPkgJson的自有main数据属性。只接受本次外部入口的精确相对路径或已恢复的原入口；前者须可写，保留原描述符并仅将内存值恢复为./application.asar/app_launcher/index.js。磁盘package保持已认证的外部入口，不重写签名应用。缺失、访问器、未知入口或不可写属性只报告固定QQ_LAUNCHER_ENTRY_RESTORE_FAILED，不输出异常正文，不据此确认身份或就绪。固定组合的正式GUI/账号验收见TASK-006；此内存兼容行为不作为其它启动器版本的通用承诺。

## 实例目录和端点

首次数据所有者使用私有容器布局：<runtime>/profile-home/Library/Containers/com.tencent.qq/Data；后继运行通过认证的 profile 引用继续使用该布局。
QQ 由 home 推导的 Group Containers 必须仍落在同一实例根下。
Electron home/appData/userData、documents/downloads/desktop/media、temp/logs 逐项显式设置到受控目录。
不能用扩大到原用户目录的读写权限解决路径失败。符号链接/祖先解析、旧路径迁移和并发写入需产品级检查。

原版默认配置名为 onebot11.json，账号配置为 onebot11_<account>.json；不能写成 onebot.json。
新实例不预填账号，不覆盖已有账号配置。业务 HTTP/WS、管理端点及凭据分离，回环监听，关闭非需求插件/CORS/调试。
上游日志留私有目录；不能用日志中的 token/账号/二维码作为公共状态输出。

## Rust 启动准备与入口生成

macos::adapter::Prepared只检查已有实例/Framework目录布局，不创建目录、复制产物、签名或启动进程。输入须为规范绝对路径，拒绝符号链接、父级跳转、控制字符与重叠根；要求Electron目录及temp/logs/config/cache/napcat-data预先存在。QQ与Framework文件验证由入口及安装/生命周期审计负责，Prepared不是运行所有权凭据。
生成全新环境集合，调用者必须env_clear后应用，不能合并继承环境：HOME与CFFIXED_USER_HOME固定为实例profile-home；TMPDIR、XDG_CONFIG_HOME、XDG_CACHE_HOME、NAPCAT_WORKDIR固定到实例目录；PATH仅系统目录，LANG=en_US.UTF-8。环境清理不等同于sandbox；实际隔离策略由start/sandbox渲染并绑定完整启动输入。
启动目标固定为专用QQ.app/Contents/MacOS/QQ，cwd为实例根；唯一Electron参数是与入口userData一致的--user-data-dir字面参数，不经shell。生成main.cjs时使用JSON序列化路径配置，固定引用同目录entry.cjs及其bridge.cjs；只导入original-fs/Electron与原创入口。路径审计使用Electron提供的original-fs读取物理文件状态，避免普通fs将application.asar虚拟化为目录；原QQ入口仍用正常require加载归档成员，不放宽真实目录、链接或多链接文件的拒绝规则。失败仅输出固定错误码并设exitCode=1，不泄漏上游异常；这不是整树终止、持久失败记录或启动成功证明。
正式start在原锁下认证版本/摘要，整组发布这三个文件，部署并复核专用副本，施加sandbox并持久化启动意图后才创建进程。Prepared仅提供纯准备输入，不单独授予启动权威。

## 原创入口资源整组发布

Prepared.publish_adapter将main.cjs/entry.cjs/bridge.cjs作为固定三文件集合发布到实例adapter目录；父目录必须预先存在、当前UID拥有且无组/其他权限。集合目录名是带域分隔和名称/长度前缀的SHA-256，包含本次路径配置及两个编译期资源；更新形成新目录，不就地覆盖旧文件。
底层storage/state/durable/bundle使用原目录描述符创建随机0700暂存目录、独占0600文件；写完各文件并F_FULLFSYNC，再同步暂存目录，复核原目录和调用方guard后，以macOS RENAME_EXCL一次发布整个目录。目标已存在即失败，不覆盖，即使内容相同；显式verify_adapter用于独立复核既有资源。
发布后同步父目录、flush文件并再次检查原目录身份与guard。任何发布后失败仍返回错误，不自动回滚、不作为成功回执；暂存失败材料保留为明确的.tmp目录，不递归删除可能被替换的路径，后续生命周期清理须独立认证。只认内容命名的完整集合，不从暂存目录启动。
verify_adapter从本程序与本次路径重新计算预期值，不信任磁盘清单。检查精确三文件集合、单链接普通文件、当前UID、私有POSIX权限、长度和逐字节内容；读文件用NOFOLLOW/NONBLOCK和目录描述符，读取后检查路径对应原inode及权限/链接数，拒绝额外文件、链接和等长篡改。
该机制只保证这些操作顺序和即时观察，不授予运行所有权、不修改QQ入口、不替代祖先ACL或恶意同用户并发修改防护，也未作断电验收。启动端仍须持有原生命周期权威并在效果边界复核所有部署材料。

## 专用副本入口配置改写计划

Prepared.package_edit接收经安装层认证的原package.json字节、预期SHA-256、版本和buildVersion；输入最多1MiB，先验证摘要，再解析唯一顶层字段的JSON对象。必须为qq-chat/darwin，原main严格等于./application.asar/app_launcher/index.js；type若存在仅接受commonjs。已改写入口、未知版本/构建及重复顶层字段拒绝，不覆盖其它加载器。
再次verify_adapter后，仅将main语义字段改成相对QQ.app/Contents/Resources/app的已发布外部入口路径（../../../../adapter/内容摘要/main.cjs），以匹配Electron的path.join解析；验证时仍绑定原绝对入口，保留其它JSON字段值，重新序列化格式；返回原/新摘要、结果字节和专用QQ.app内的固定目标路径。PackageEdit不实现Debug/Serialize，避免默认打印私有路径；verify_source/verify_result供部署边界复核，生成计划不写文件也不授予写入权限。
预期摘要由调用者提供，必须来自已认证的官方DMG/安装审计；不能把对任意输入自算摘要视为认证。原package的eleArch不是原生架构证据，保留其值且独立检查Mach-O。受控副本的实际入口部署及身份/锁复核见下方“副本入口部署”；签名、d2只读恢复与启动器运行期main兼容分别由相应层组合。

## macOS QQ 选定载荷审计

qq::inspect在macOS arm64对名为QQ的可执行路径转交qq::macos，原QQ.exe检查保留。只接受.app/Contents/MacOS/QQ布局和原始QQ入口，检查QQ、major.node、wrapper.node、application.asar、package.json、Info.plist六项；每项限512MiB，两个元数据文件各限1MiB，总量限1GiB。不访问账号存储，不执行QQ或原生模块。
Mach-O解析接受little-endian 64位thin及big-endian fat32/fat64容器，最多16个slice；检查表界限、对齐、非重叠、唯一arm64及表/内部subtype一致，拒绝仅Intel和arm64e。限定当前形状为主程序MH_EXECUTE、major MH_BUNDLE、wrapper MH_DYLIB；加载命令数/总长有界并检查逐项长度与对齐。此为头部/命令形状检查，不证明所有加载段、依赖、代码签名或程序可运行。
各文件以NOFOLLOW/NONBLOCK打开并保留描述符，拒绝硬链接，读取前后复核路径的dev/inode/长度/模式/属主/时间戳；哈希使用原描述符并检查截止与长度。package要求qq-chat/darwin、固定原main、版本与buildVersion一致；eleArch不参与架构判定。复用ASAR目录检查确认原入口成员边界，不解包或执行载荷。
Application保留通用字段；metadata_sha256通过独立macOS域编码绑定wrapper、package和Info.plist摘要，应用fingerprint绑定版本/构建/arm64及全部选定文件。路径id不小写归一化。它是选定文件的即时指纹，不是完整.app封印、可信下载证明、所有权或启动许可；QQNT.framework、helper、完整签名与来源认证仍须部署层补齐。

## 官方源应用静态签名

native::macos_signature使用security-framework 3.7.0与core-foundation 0.10.1安全接口，不新增unsafe。verify_original对规范绝对.app目录执行SecStaticCodeCheckValidity，开启STRICT_VALIDATE、CHECK_NESTED_CODE、CHECK_ALL_ARCHITECTURES及NO_NETWORK_ACCESS；不关闭可执行文件或资源验证。校验原根目录身份/模式/属主/时间戳，并在调用前后检查截止。
来源要求分开记录：TencentDeveloperId要求Apple generic anchor、QQ identifier、Developer ID中间/叶证书OID及腾讯OU=FN2V63AD2J；AppleAppStore要求Apple generic anchor、QQ identifier及商店叶证书OID。商店版不能按Developer ID叶证书OU误判，也不能把ad-hoc包当官方源。安装层仍须按固定DMG路线选择来源，签名分类不自动扩展商店版部署支持。
qq::macos::inspect_original先取得选定文件指纹、再检查签名、最后重读选定指纹并要求一致，返回不可序列化的Original观察及来源。普通inspect保持纯形状/指纹能力，合成文件不因可解析而通过官方源检查。
验证不访问Keychain密码条目，不启动或重签QQ。NO_NETWORK_ACCESS不提供实时吊销/在线公证/Gatekeeper结论；全架构检查保护原通用包的签名完整性，不增加Intel运行支持。系统同步调用仍不可在原线程中取消；inspect_original现使用下述隔离工作进程，父进程不再等其系统调用返回后才处理超时。签名观察也不替代来源下载摘要、专用副本授权和效果前复核。

## 签名检查工作进程

inspect_original使用verify_isolated执行签名步骤。仅重启当前CLI可执行文件，使用私有版本化工作标志、目标路径与毫秒预算；不经shell、不接受用户指定检查器，不继承环境，stdout/stderr丢弃。主入口在公开参数解析前识别该固定私有模式，不加入公开帮助或Schema；工作模式无状态/凭据/QQ生命周期访问。
签名工作进程限300秒，只有四个固定退出码表达Developer ID、商店、失败与超时，不解析原生错误文字或标准流。父进程5ms有界轮询，仅接受截止前的预期结果；超时关闭存活管道，并对仍由本次std::process::Child持有、未回收的直接子进程请求终止，最多另用2秒观察回收。查询错误不据旧PID发信号；无法确认退出返回SIGNATURE_WORKER_CLEANUP_UNKNOWN，不宣称检查成功或已清理。
工作进程自身有独立截止线程，另有stdin存活管道观察线程；超时或父端关闭即结束这个只读工作进程，即使主线程仍在Security Framework内。不终止QQ或系统信任服务，不把此临时Child机制用于持久QQ运行/PID恢复。子进程不派生业务后代，管道不传送凭据。
此处将同步库调用的等待隔离，正常截止与回收宽限均可验证；不能承诺操作系统调度、spawn或不可中断内核状态下的绝对墙钟上限。宿主若作为库使用该组合接口，其当前可执行文件也必须注册同一私有worker入口，否则拒绝结果，不回退同步验证。

## 应用树内容与扩展属性基线

install::macos::bundle::Inventory覆盖整个.app的目录、普通文件及相对符号链接，不沿符号链接递归。根必须规范绝对、非链接目录；每个节点同设备，拒绝特殊文件、硬链接、setuid/setgid/sticky及组/其他用户可写的目录。普通来源文件的可写位原样记录并单独统计，不静默收紧；观察到真实QQ有5个0666 shader文件。清单不是原位启动许可，专用副本部署须收紧权限。链接目标须可解析且留在包内，拒绝绝对/失效/循环解析/越界链接；合法Framework的Versions/Current等相对别名保留。
限制2万节点、64层、单文件2GiB、普通文件总量4GiB及路径/链接长度4096字节；检查截止。实际已安装QQ约1.38GB，不能误用此前选定载荷的1GiB总量上限作为整个包限制。遍历与普通文件打开使用保留目录描述符和NOFOLLOW；用原文件描述符哈希，在结束时重新核对全树节点dev/inode/大小/权限/属主/链接数/修改与变更时间，发现并发变化即拒绝。读取缓冲区放堆，避免最大深度耗尽线程栈。
清单指纹使用域分隔、相对名称、类型、POSIX权限、文件长度/内容摘要和链接原文本；无Debug/Serialize，不默认输出本地名称。revalidate同时要求原身份与内容；verify_copy对独立非嵌套根比较完整内容清单，并复核原来源的节点身份。源与副本的inode/UID/时间戳不要求相等。
扩展属性由native::macos_xattr窄只读边界对已核对身份的节点描述符调用flistxattr/fgetxattr读取，两者固定启用XATTR_SHOWCOMPRESSION，枚举并读取默认隐藏的压缩属性；不启用XATTR_NOSECURITY或权限绕过，普通文件和目录使用NOFOLLOW，Darwin链接使用O_SYMLINK打开链接自身而不同时设置NOFOLLOW；不读取目标属性替代链接属性。按属性名排序记录长度/SHA-256，读取后复核名称集；任何读取失败、数量/长度变化或超限均拒绝，不静默遗漏。每节点名称列表限64KiB、256项、单名称255字节；普通属性单值16MiB，全树名称和普通值累计64MiB；资源叉单独限2GiB/项、4GiB/树，以64KiB缓冲区按资源叉position分块哈希，每块检查截止与实际长度，结束重查总长，不一次分配完整资源叉。仅存摘要，不提供原值、Debug或Serialize。资源叉和quarantine按系统枚举出的属性参与原树重核及副本比较。
指纹域为qq-cli-macos-app-tree-v4，包含扩展属性、ACL摘要及st_flags；此内部清单没有旧版本持久化迁移或重新签署路径。ACL通过native::macos_acl窄只读边界对已打开节点调用acl_get_fd_np(ACL_TYPE_EXTENDED)，使用acl_size及acl_copy_ext读取至多64KiB的大端外部表示，再计算域分隔SHA-256；保留条目顺序、主体、权限和继承标志，不解析用户名或输出主体标识。持有描述符时ENOENT表示缺少扩展ACL并使用独立摘要域，其他失败拒绝；SDK分配由RAII释放。O_SYMLINK描述符读取链接自身ACL。文件系统st_flags加入原身份复核和副本比较，不静默清除来源标志。清单只证明元数据一致，不证明ACL安全或批准执行；副本权限收紧和不可变/压缩等标志策略仍由部署层实现。压缩属性按名称适用普通属性或资源叉预算，使用相同摘要比较规则；逻辑内容相同但压缩表示不同仍拒绝。枚举范围受系统访问权限限制，不承诺能观察系统未授权暴露的元数据。生产复制须保留并核验适用元数据，继续执行来源签名、路径权限和原锁检查；不能以此清单或合成ditto复制替代生产部署完成。

## 受控原样副本复制

Inventory.copy_to_new只复制已建立清单的来源，不认证官方来源、不改入口、不签名或启动。调用方提供既有私有父目录和原生命周期guard；父目录须规范绝对、无链接、当前UID拥有、无组/其他权限且无扩展ACL。源与目标父目录不能重叠；目标为单个.app叶名称。原样复制目前允许UF_COMPRESSED、UF_HIDDEN、UF_NODUMP及零标志，未知/系统/不可变/追加标志在任何写入前拒绝，不静默清除。当前安装树只读观察为零或UF_COMPRESSED，不作为所有未来包的保证。
先复核来源清单与原父目录身份，再以描述符相对mkdirat独占创建随机0700的.partial.app暂存根。逐节点使用原目录描述符打开来源、核对原stamp，独占创建目标；不沿链接递归，链接文本来自已核对清单。目录先复制子节点，最后复制目录元数据，避免继承ACL影响本来不含该ACL的子节点。native/macos/copy.rs使用fcopyfile复制数据/元数据，状态由RAII释放；只持有同步描述符，不经shell、不递归调用路径复制、不启用MOVE/UNLINK/RUN_IN_PLACE。同步内核的截止回调与每节点前后检查为协作式取消，不能中断阻塞内核调用；生产装配使用下节的隔离入口。
fcopyfile会重建quarantine的来源/时间字段。属性层仅对清单中已有quarantine读取原值并核对长度/SHA-256，再按原字节写入新副本；不清除隔离标记，不修改来源或系统Gatekeeper设置。普通文件F_FULLFSYNC，目录fsync；保留原文件模式和ACL，包括普通来源0666模式，因此原样副本不能直接当作加固后的可执行部署。
暂存树完成后核对完整内容/属性/ACL/标志及持有根身份，再复核来源、暂存树和guard，以RENAME_EXCL一次发布；已有目标不覆盖。发布后同步父目录，复核父身份、guard、完整树、当前UID和节点身份，根因rename产生的时间变化单独处理。每次guard前后检查父目录，路径替换拒绝。错误保留暂存或已发布但未确认的副本，不递归删除或宣称回滚成功；生命周期日志和清理接合前不能自动重试为一次成功安装。该流程不证明祖先ACL策略、恶意同用户隔离、断电恢复、生产DMG获取或最终权限加固完成。

复制核验允许来源不存在时新增1至4096字节的com.apple.provenance；仅将其视为系统附加的不可解释元数据，不删除、写入或用于授权。来源已有的所有属性（含quarantine、资源叉及既有provenance）仍须逐字节一致；副本最终清单独立绑定新增属性。

## 持久复制工作进程

生产装配应使用Inventory.copy_to_new_isolated；同步copy_to_new保留为明确的协作式内核与合成对照，不当作已隔离的启动路径。隔离入口在父进程建立一个Worker，通过当前CLI的私有--internal-macos-copy-v1模式启动，env_clear、标准输出/错误丢弃；只接受1至1800000毫秒预算，不接受替代工作程序、来源/目标路径、账号或凭据。嵌入此库的宿主须在公开参数解析前注册worker_entry，否则拒绝，不回退同步复制。
父子使用匿名UnixStream及SCM_RIGHTS传递能力。每个复制请求为一个固定类型字节和恰好两个描述符；结束请求无描述符。协议拒绝截断、额外/缺失描述符和未知类型，接收描述符立即CLOEXEC并由OwnedFd回收。双方检查不同dev/inode、相同受支持节点类型、目标当前UID；数据请求要求普通文件、目标单链接且空、共享文件偏移为零。链接用已打开的O_SYMLINK描述符，不另行解析路径。每次收到成功字节只表示该节点的fcopyfile已返回，不授予完整副本身份。
父进程等待时每5ms核对截止、原目录和调用方guard；失锁、错误、超时立即关闭通道，只对原Child当前确认尚未回收的直接子进程终止，并最多额外2秒观察退出。无法确认回收返回COPY_WORKER_CLEANUP_UNKNOWN，不复用该Worker；清理只尝试一次，避免析构重复延长宽限。查询错误不据旧PID发信号。丢弃未结束Worker也执行有界回收，但不能提供调用方可消费的成功结果。
子进程有独立总截止看门狗和不消费请求的父连接观察线程，复制阻塞时仍可因截止或父通道EOF退出。正常整树复制结束后，父进程发送结束请求并确认固定成功退出码，再继续完整树核验及无覆盖发布；任何未知退出码或缺失回复拒绝。整个树复用同一个工作进程，不为每节点重启程序。没有QQ进程树操作，也不把该临时Child机制当作QQ生命周期权威。
隔离覆盖阻塞fcopyfile调用。来源/副本清单、quarantine原值保留、文件同步、目录核验和发布仍在父进程执行，保留协作式截止；不承诺操作系统调度/不可中断系统调用下的绝对墙钟上限。原样副本仍需最终ACL/模式加固、日志与清理接合，不能据此宣称生产启动完成。

## 私有副本权限加固

受控复制成功后，Inventory在内存中持有原私有父目录描述符作为复制能力；普通inspect/verify_copy不能生成或恢复该能力，不序列化此能力。harden_copy消费该对象，仅允许已完整核验的新副本；再次检查父身份、当前UID/私有模式/无ACL、guard及原树stamp后才修改。未经复制的原安装、仅重新扫描的私有路径或已变化副本均拒绝。
每节点通过保留目录描述符打开，检查原dev/inode/属主/模式等stamp，先处理子节点再处理目录。native/macos/acl.rs新增明确的owned ACL清除：用acl_init(0)/acl_set_fd_np作用于已打开且当前UID拥有的普通文件、目录或链接，随后确认扩展ACL确实缺失；链接用O_SYMLINK，不清除目标ACL。普通文件必须单链接。没有路径chmod或递归shell操作。
目录统一0700；普通文件为0600加来源原所有者执行位；链接POSIX模式保留，清除链接自身ACL。来源内容、相对链接文本、全部属性（含quarantine/压缩资源叉）、文件系统标志及节点身份/属主/链接数均须保持。逐节点在ACL和模式修改边界核对guard；普通文件完整同步、目录同步。完成后重新扫描整树，逐项验证只发生预期ACL/模式变化，并核对节点身份、大小与mtime及原父目录（ctime随权限变更更新），再返回新的指纹/复制能力。指纹格式仍为v4，仅权限和ACL值改变。
失败保留可能部分加固的副本，消费的能力不作为成功回执返还，不自动放宽或回滚权限。此阶段不恢复磁盘安装记录的写权限，也不替代后续入口部署、签名、升级恢复或启动授权。ACL/模式及同步调用在父进程执行，仍为协作式截止；祖先权限与同用户隔离不因私有父目录检查而被宣称完成。

## Rust 平台接口

通用编排依赖平台能力：准备启动、验证进程身份、验证同一连接归属、观察退出、停止已拥有运行。
macOS 身份使用独立平台标记，不复用 Windows FILETIME/Job 编码；旧记录只在其原有能力范围读取，不自动迁移或补造权限。
当前实现使用 libproc/kqueue 观察、audit token 信号、Keychain、同 socket 归属及继承执行限制；持久权威由下述签名协议提供。
PID 重用、父退出、子进程脱离、端口转移、凭据恢复、停止前重核及失败清理的完整矩阵见 TASK-006。

保留同一 socket 的归属核验后再发送认证，不能把一次 lsof 监听查询或 URL/token 当作产品受检连接。
安装发现、DMG/app处理、签名、兼容诊断与生命周期通过平台模块组合，OneBot/领域层复用通用协议；arm64 CI与安装分发机制见SPEC-005。


macOS自工作进程统一使用native::process::current_executable：通过本进程Pinned观察读取内核镜像路径，再检查无链接普通文件并复核原进程。复制、SDK校验/Keychain授权、签名监管及磁盘attach/release均复用此入口；不使用std::env::current_exe可能保留的启动符号链接，也不重新解析可能已切换到其它版本的命令入口。该观察不是载入代码字节证明；安装器保留不可变版本槽及旧程序，使原操作仍可启动同版本工作进程。受管链接启动及运行中切换入口纳入合成回归，脚本装配门禁也经符号链接入口执行。

## 进程身份

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
OneBot/WebUI 受检 HTTP 和 WS 复用通用协议实现，经平台 guard 验证原 socket 后才写认证，响应/握手后再次复核。macOS guard输入由下述j2/s1认证运行状态提供；单独构造 Pinned 不是 CLI 自动接入许可。

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
结果仅为动态候选快照，不授予所有权、停止权限或“整树为空”的完成凭据。父退出/更换进程组不影响文件查询，但两次扫描间仍可 fork/exec；实际映像内容也不能仅凭路径认定。CLI stop通过下述j2/s1/d2认证文件与启动来源、限制可执行范围，并以p2持久意图组织成员冻结及终止收敛；候选发现本身不授予这些能力。
FFI 仍限原有 macos_process 边界，census.rs/census.c 不新增终止能力或全局进程元数据输出。

## macOS 凭据适配

SystemCredentials 使用固定 security-framework 3.7.0 的安全 Rust API访问本机非云同步 Generic Password，service 继续由规范化状态目录摘要派生，account为已有逻辑凭据键。macOS 不套用Windows目录重定向的legacy namespace回退；空键、控制字符和过长键在接触Keychain前拒绝。
读取、创建/更新和删除分别检查系统返回值；只有errSecItemNotFound映射为缺失或幂等删除。访问拒绝、锁定及需交互等错误明确失败，不回退明文、不替换已有签名权威。
Keychain交互开关属于进程全局状态；本适配用进程内Mutex串行保护，操作期间禁用系统交互，只在原先允许交互时创建恢复guard。该Mutex不代替跨CLI状态/生命周期锁，原签名密钥初始化仍依赖Store事务。
Keychain条目受创建程序访问身份约束；同一程序可访问不代表不同签名构建或升级后的CLI自动获得访问。升级后若被系统拒绝，通过下述显式入口请求用户授权，并以随后新进程的默认无交互访问确认恢复；不要求用户提供Developer ID证书，不复制或替换既有密钥。钥匙串锁定、拒绝及全部升级组合仍须按各自范围验收。


显式napcat start --authorize-keychain在原正常启动前请求系统授权，默认false且仅原生macOS arm64提供。app/runtime/macos/authorization按选中实例/代次派生固定signer、runtime-bundle和runtime-api键，拒绝登记的异域凭据键；新实例只请求已有signer。授权期间不持有状态事务，原状态锁身份和完整Registry快照由guard在工作进程前后及等待中重核；变化即拒绝继续启动。

storage/credentials/macos/authorization复用现有SDK工作进程监管：只执行当前CLI、清空环境、固定service哈希和一组同代次逻辑键，输出和错误均丢弃。显式人工授权遵守原命令剩余timeout，最多1800秒；父端监管与子端看门狗使用同一预算，不另截为300秒。普通来源/权益签名检查仍保留300秒上限。超时只报告失败，用户完成残留窗口不能替代新进程的无交互复查；未确认退出报告CREDENTIAL_AUTHORIZATION_UNKNOWN，不接受凭据结果。工作进程仅用原系统允许的交互状态读取Generic Password，不启用被系统禁用的交互，不创建/删除/迁移条目或调用ACL修改。签名键必须存在，尚未准备的运行材料可缺失。值不传回父进程，也不出现在参数或日志中。

系统授权由用户选择Always Allow；工作进程成功后父进程通过默认无交互适配器重读相同固定条目，Allow Once等不足以恢复普通访问的结果仍明确失败。随后才进入正常References、兼容、来源和一次派发检查；显式授权不产生兼容证据。真实跨构建拒绝→用户授权→原权威和材料恢复的限定验收见TASK-006，不推广为所有签名组合。

## macOS 状态发布与落盘

atomic_json在macOS arm64通过锁定rustix 1.1.5安全API逐层打开目录，要求每层为非符号链接目录；后续创建与renameat相对于保留的目录描述符。拒绝包含父级跳转的路径、已有非普通文件或多硬链接目标。临时文件独占创建且模式0600，序列化失败清理临时文件并保留旧目标。
发布顺序为写入完整JSON、F_FULLFSYNC临时文件、复核目录身份、同目录renameat、fsync目录、再次F_FULLFSYNC发布文件并复核目录身份。重命名后同步失败仍返回错误，不把可见文件当成成功凭据；调用方不得因保存失败继续真实操作。
References签名材料使用同一固定目录句柄独占创建0600文件，目录同步及F_FULLFSYNC完成后才返回引用。新增目录逐层mkdirat(0700)，不修改已有父目录权限；每次新增目录同步其父目录项，之后文件发布的设备flush覆盖此顺序。
权限校验集中在storage/state/durable/permissions：发布目录须为当前UID拥有且无组/其他写权限、无扩展ACL；既有目标及锁文件须为当前UID拥有的单链接普通文件、无额外写权限且无ACL；新发布文件及不可变适配资源另要求无组/其他权限。序列化前后和替换前复核原描述符；替换前发现ACL变化时拒绝继续，不自动清除或修复ACL，并保留旧目标。不可变资源读取前后也复核ACL。此为写入/读取检查点，不是断电实测或恶意同用户隔离；其它祖先目录策略及全体读取路径仍须单独完成。Windows原有实现保持。

## macOS 状态与生命周期锁

state.lock与startup.lock通过同一已固定目录描述符openat，先打开已有文件；缺失时独占创建，遇到另一进程先创建的EEXIST则重新打开并复核。使用NOFOLLOW/NONBLOCK/CLOEXEC、创建模式0600，不截断既有内容。锁目录须属于当前有效UID且POSIX模式不允许组/其他用户写入；锁文件须为当前UID拥有的单链接普通文件，且不允许组/其他用户写入。目录和锁文件均须无扩展ACL；已有无ACL的0644锁文件可继续使用，不自动chmod、清除ACL或接管其它用户文件。
等待之前、每次重试及成功取得锁之后，复核目录路径对应原设备/inode、锁文件路径对应原文件、属主/模式/链接数。竞争沿用STATE_BUSY/START_BUSY且受截止约束；Drop只关闭句柄，不删除锁文件。该检查能发现原锁被替换，不是对恶意同用户删锁的排他防护。
状态事务读取后再次复核锁，持久化使用原锁目录句柄；序列化后、rename之前再次复核锁，发布后仍复核。丢失原锁不得作为成功提交。生命周期调用已使用相同获取逻辑，macOS真实进程操作在效果边界继续复核能力，不能仅凭取得过锁就授予运行权威。
start_intent的真实效果回调之前重新核对原状态锁，失败则不调用回调。回调之后、回执保存前后再次核对；此时失锁返回START_OUTCOME_UNKNOWN并附原request_id，不宣称可安全重试。回调后的第一次重核失败时不尝试写回执；原Pending意图保留，恢复时只转Unknown，不重新发放启动机会。该共享检查继续由macOS受控start/s1调用，平台运行记录另使用j2。
上述锁与发布检查同时覆盖POSIX属主/写权限和原描述符的ACL；其它祖先目录的可写策略、全体读取路径与完整运行恢复仍需单独验收。

## macOS 服务记录的只读恢复

macOS服务绑定使用独立macos-runs索引和j2签名引用，不读取、迁移或重签Windows j1/旧t1记录。索引按(instance_id,run_id)摘要寻址，pointer schemaVersion=1；签名正文schemaVersion=1且platform只能为macos，包含原requestId、完整start_intent Target、原服务进程Identity、固定HTTP与可选WS端点。
恢复必须持有原状态锁，要求当前注册表中实例与安装分别唯一且与调用快照相同，实例处于starting/login_required/ready，安装摘要、运行代次和两个端点与签名记录完全匹配。原s1启动回执须为Dispatched并具有同一完整Target，包括inputHash；Pending/Unknown不能恢复服务。然后复核平台身份并重开原Pinned，返回前再次核对状态锁。签名和关联校验前不接触端点；记录不存在时不读取凭据、不创建状态。
应用backend从该只读能力构造macOS OwnedOneBot与SocketSource；每个HTTP/WS连接继续核对实际服务端socket后才发送认证，WS重连不降级为无归属连接。恢复本身不确认账号、修改ready、发现新服务或授予进程树停止权限。
生产start在原s1派发下调用run/publication发布j2，绑定专用副本的原根、固定端点、声明的软件组合及继承执行范围；控制材料从认证r2恢复。此生产者与只读服务恢复分开，不接受用户提供的PID/URL，也不能把任意Pinned加上Dispatched回执视为允许接管普通QQ。旧记录不迁移或补签为当前运行权威。
签名材料遗失时macos-runs索引也阻止创建替代密钥，即使references目录已缺失。

## 验收界限

原型CHECK-072与正式CLI CHECK-136分别保留当次方法和限制；当前已验收功能及剩余平台/故障/公开交付范围仅由TASK-006维护，不从原型或单个接口成功推导全部兼容。
不支持 Intel/Rosetta；最低 macOS 版本未由单机实验确定。OCR/WebUI 终端的上游限制见 FACT-005。

## 固定输入生产者与实例装配

runtime/macos::assembly::Plan在既有prepared运行布局上持有原生命周期锁，读取唯一实例/安装及原状态锁身份，拒绝账号/进程标记，要求所选macOS目录版本的固定摘要，并通过i1安装记录完整核验文件。复核私有构造的固定Source，派生runtime_store同一实例/代次目录，生成AdapterPlan及认证原package计划；返回前再次核对原状态锁、完整实例/安装快照及生命周期锁。规划不发布adapter/app，不创建凭据或进程。
输入摘要采用qq-cli-macos-deployment-input-v2域分隔及固定结构JSON，绑定l2目录路径/凭据指纹、完整实例/安装快照、Source选定载荷及原执行路径、完整原树、固定DMG URL/长度/摘要、Framework版本/提交/URL、显式角色清单摘要、适配组摘要/入口、package原/结果摘要及目的路径、工作目录/执行路径、字面参数和排序环境。角色摘要另以长度前缀编码全部路径/标识/执行名/角色及精确权益模板。私有Plan不能从一个hash或序列化输入恢复；摘要不是独立认证凭据。
Plan.deploy先复核原store/状态锁身份及快照，再调用macos/deploy的d2一次意图；新意图回调内重核Framework、Source和布局，并使用同一guard执行Assembly。顺序固定为创建私有adapter父目录、整组无覆盖发布、隔离复制、加固、部署package、在副本当前清单上建立角色计划、受监管签名/权益验证/同步。发布后guard同时精确核验适配组；长操作不持续占用状态锁。最后再完整核验原Source、Framework及状态快照，才把签名Inventory交给d2消费为完成凭据。任一步失败保留现场，由d2记录Unknown或保留Pending，不自动回滚/重放。
Assembly是低层效果组合器，只接受调用者已认证的清单/角色/计划与guard；它自身不宣称上游来源或创建持久意图，生产入口为Plan。该分层用于原创合成验收；不能以Assembly的可调用性跳过固定来源、i1认证及d2日志。Plan.revalidate只读检查，不授予启动权限。已有Complete/Unknown记录继续按d2规则处理，不重建内存owner。返回Complete前（包括已有完成记录）仍独立核验Framework的i1完整文件与实际适配组，并复核状态快照；不重新运行原回调，d2应用凭据本身不证明外部依赖永久未变。正式CLI通过下述start组合该Plan、隔离启动及认证运行发布；失败清理和父端阻塞文件系统预算的差距见TASK-006。

## 部署效果前的入口计划

Prepared.adapter_plan只根据当前准备路径和编译内嵌的原创入口生成AdapterPlan，包含三文件组摘要、固定发布目录和main入口路径；不读取/创建adapter目录，不把摘要或计划对象当作已发布证明。publish继续使用原来的整组无覆盖发布和guard；verify按该计划精确核对目录和字节。Prepared.publish_adapter/verify_adapter委托同一计划生成逻辑，防止计算与发布采用两套内容。
Prepared.plan_package_edit允许在适配器尚未发布时计算原package仅main改变的完整结果字节及原/结果摘要；仍检查原摘要、唯一JSON字段、固定入口、版本/构建等，调用者传入的原摘要不自动证明来源。旧package_edit在相同纯计划之外继续要求verify_adapter。生产装配使用plan_source_package，从私有构造的macos::source::Source取得受认证package及版本，避免用调用者任意文本冒充固定官网元数据。
Source.package_bytes委托原Inventory，仅对固定Contents/Resources/app/package.json经原目录/文件描述符有界读取（1MiB），比对长度/内容摘要/原stamp，并在读前后核对整树stamp；没有普通路径重读或由文件自报摘要。它不是新的完整树验收，部署前后仍需Source完整重核。部署方法继续在真实写入边界verify_adapter；纯计划不能越过发布、活跃副本或生命周期检查。此拆分使完整输入摘要可在Pending之前计算；Framework认证、摘要组合及完整装配调用由assembly::Plan负责。

## 副本入口部署

Prepared.deploy_package将PackageEdit绑定到当前Prepared生成的适配入口与固定QQ.app目标；每次效果guard后重新核对三文件适配资源。安装层仅接收固定package路径、原摘要、结果字节和guard，不依赖运行时模块。
部署消费受控复制返回的内存能力，要求整树为当前UID、无ACL、目录0700及普通文件0600加所有者执行位。普通扫描不能授予写权限。先核对整树及原package摘要，再逐层保留目录描述符，以NOFOLLOW打开原package且不在打开时截断。可写打开前再次核对原锁/父目录、适配资源、整树stamp及只读打开文件身份；可写打开本身也属于效果边界。原位写入完整字节、截到准确长度并F_FULLFSYNC；失败可留下部分内容，消费能力、不自动回滚或重试。
入口写入完成后重扫整树，相对写入前（压缩目标为解压后）清单仅允许package内容/长度及mtime/ctime变化，保留其inode、属主、模式、ACL、属性和标志；其余节点内容和stamp必须完全相同。最终再次核对guard、适配资源和stamp才返回新清单。成功仍不是签名或启动凭据。对于UF_COMPRESSED的package，由系统可写打开执行解压；不手动删除压缩属性或quarantine。立即完整同步并重扫整树，要求目标逻辑内容、inode、权限/ACL、属主和链接数不变，仅允许移除UF_COMPRESSED、com.apple.decmpfs、系统移除的ResourceFork及时间戳变化。仍存在的ResourceFork必须原字节一致，其他属性含quarantine必须全部保持，不接受新增属性；其余节点内容和stamp不变。解压核验后再次核对guard和stamp，才写入main改动。解压失败或失锁不继续、不返回能力；可能留下已解压但尚未改入口的副本。应用其他压缩资源不修改。文件系统操作仍为协作式截止，不宣称断电原子替换、持久恢复或恶意同用户隔离。

## 受控运行目录准备

runtime/macos/layout在原生命周期锁和短状态事务下，为唯一实例/代次/安装摘要写入l2 Pending后创建固定目录。仅接受无账号/进程的preparing或prepared实例，既有无凭据目录不接管；不提升实例状态，不生成凭据、配置或启动进程。索引位于macos-layouts，原签名密钥丢失不得自动重建。
目录树由macos/adapter共享布局派生，以描述符相对mkdir/open创建；固定目录须为当前UID、0700、无ACL/flags且与状态目录同设备。持有原目录句柄，在创建和同步边界重开核对路径身份；完成前要求准确空布局并同步目录/父目录。POSIX创建与打开并非原子操作，不宣称抵御恶意同用户竞态或完成断电验证。
成功保存Complete及固定父目录/节点device/inode/UID，失败保存Unknown；失锁保留Pending。相同请求只读恢复，不重复效果；不同请求拒绝。恢复允许后续普通内容，但继续检查固定目录身份和权限；这不是任意子内容的认证、递归清理授权或运行权限。
Plan.inspect必须恢复Ready布局；输入v2绑定目录和记录指纹，效果guard、只读重核及返回Complete均复核固定目录。手工创建同名目录不能替代l2凭据。配置/凭据准备和CLI路由由下述material/start接合；失败现场清理不由l2只读恢复授权。

## macOS 首次配置与凭据准备

固定Framework提交859eb530c5801577633c2c3ae871d23586edf281的NapCatPathWrapper将config/logs/cache/plugins置于NAPCAT_WORKDIR下；因此本平台配置目的地为<instance>/napcat-data/config，实例根config仅供XDG_CONFIG_HOME，不能混用。固定源码依据与观察边界见FACT-005，不代替发布包运行验收。
l2布局版本2加入四个NapCat子目录；版本1凭据拒绝且保留原树，不隐式迁移或接管。macos/config复用runtime_config::Bundle的所选目录版本模板，在认证Layout的config空目录中以描述符CREATE|EXCL创建0600、无ACL、单链接普通文件。仅发布napcat.json、onebot11.json、webui.json；既有默认/账号配置或额外文件均拒绝。写入及同步前后调用上层guard和目录身份复核，文件F_FULLFSYNC、目录fsync，失败保留现场、不覆盖/修复。
Published只记录布局指纹、文件身份和摘要，不含token；verify在首次启动前匹配Bundle及实际精确字节/权限/身份。反序列化Published本身不产生认证，由macos/material的r2完成记录绑定；运行后可变配置另设观察策略。此模块是低层文件效果内核，不访问Keychain、不提升实例状态、不授予启动权限。
runtime/macos/material以r2记录绑定规范Plan（已选目录版本、三个端口）、原实例/安装快照、l2指纹和原始随机凭据包摘要；先检查空配置目录与两个credential key均不存在，再发布Pending。生命周期锁序列化协作客户端，状态锁仅用于短事务；Keychain读写及文件核验不持有状态事务。效果guard复核原锁身份、快照、l2目录和原Pending。
凭据包含目标绑定摘要及独立API/control/JWT随机值，保存在runtime-bundle键；业务API另存runtime-api键供既有传输使用。r2记录只含随机凭据包摘要，不含明文token。每次写入前检查键不存在，写后读取比对；Credentials::set不是CAS，不宣称恶意同用户隔离。失败不删除已写键、不创建替代密钥；原签名权威缺失时macos-materials索引阻止重建签名密钥。
全部配置及凭据核验后记录Complete，再将preparing提升为prepared。若只完成记录而未完成状态提交，相同请求可以验证原材料后补状态提交；已prepared的重复请求不改注册表。recover始终只读，Pending/Unknown返回未知，不接续部分效果；不同请求拒绝。已有Complete仍核验原签名、布局、凭据摘要/API一致性及精确配置，缺失或变化拒绝而不修复。
准备不证明Framework安装字节、QQ来源或部署签名，后续装配继续i1/Source/d2检查；不授予启动权限。正式CLI通过start接合，运行后管理凭据从原r2只读恢复；显式Keychain授权见上节，失败现场清理的差距见TASK-006。

## 固定来源下的首次实例预约

macos::creation::reserve仅预约实例和三个回环端口，不产生启动兼容性正例。入口要求私有构造的固定官网Source、唯一已选macOS目录版本安装以及i1完整校验；持有原生命周期锁，在长文件检查外使用短状态事务保存并复核原状态锁身份和安装快照，预约后再次复核来源/安装。此步骤不复制、签署或启动QQ，不创建目录或凭据。
实例登记协议复用instance_creation::reserve_in，以Admission提供已检查安装、固定release、签名类型及生命周期guard。macOS使用n2，旧入口及Windows继续n1；索引同处instance-creations，签名类型不匹配拒绝，不隐式转换。已有记录绑定请求、安装、版本、实例/代次及端口，重复请求返回原实例；已登记实例被移除时拒绝重建。Reserved发布先于注册表写入；中断只恢复原ID与端口，端口不可用时不另选。
状态锁身份在协议效果边界复核，guard在意图发布、注册表持久化和结果返回周围检查；相同ID重复或代次被其他实例占用拒绝。Fresh持有端口Lease，Existing不新建租约；后续启动仍须按原端口重新确认并安排释放时序。checkpoint仅只读读取n2，不授予运行权限。
此入口由正式start与l2、r2、assembly、兼容门禁、隔离派发及运行记录组合；n2预约本身不证明完整启动或在线账号。

## 固定 DMG 获取与缓存

macos::source::image::Image对固定DMG长度/SHA-256进行描述符读取，并持有原文件身份、大小、mtime/ctime及quarantine原字节；重核同时比较原句柄和当前路径，拒绝别名、符号链接、同字节替换和文件变化。此对象仅证明镜像字节，不是挂载回执或Source，也不授予挂载权限。
image::acquire在独立qq-image.lock下使用qq-images固定摘要目录文件；有效缓存完整重核后复用，不打开新的来源。缓存文件须为当前UID、0600、无ACL/flags、单链接普通文件，目录0700。新输入来自显式本地文件或固定官网HTTPS；无代理/自动重试，只跟随有界qqdl.gtimg.cn同源HTTPS重定向，拒绝凭据、非443端口、fragment、非200、错误声明长度与内容编码，读取仍受精确总长度/摘要约束。
固定partial槽位CREATE|EXCL，读写前后复核原锁/父目录/文件身份，完整流校验和重新读取后以NOREPLACE发布，目录fsync及文件F_FULLFSYNC。正常失败仅清理本次仍同身份的临时文件；失锁、父目录变化或进程中断保留现场，不接管或覆盖。固定槽位限制遗留文件数量；已有完整partial仅在固定长度/摘要、私有权限及原身份复核通过后发布，保留inode，不重新打开来源。不完整或变化现场保留并拒绝，不追加、覆盖或自动删除；缓存获取层不授予持久清理权限，不启动磁盘挂载工具或QQ；只读挂载监管由下述m2流程负责。
本地输入从认证描述符读取，在任何内容写入前原样复制quarantine并F_FULLFSYNC；EOF重新核验原输入，发布前后比较暂存及目标属性。Image重核拒绝quarantine变化，不清除该属性。有效缓存优先复用时不读取或合并新传入来源的属性；HTTP输入无本地源属性可复制，不伪造已有下载器标记。镜像观察不是Gatekeeper许可，后续挂载/应用验收仍须遵守系统策略。


## 只读挂载观察

image/mounted观察固定Image对应的既有挂载，不执行attach/detach，不产生挂载所有权。先后复核固定镜像，通过固定系统hdiutil info -plist读取有界原始报告，以固定MIT许可plist 1.8.0的安全Rust接口反序列化所需字段；无关Data/Date元数据按未知字段忽略，必需身份字段仍严格检查类型，不先转JSON；系统命令清空环境、不经shell、不接受用户选择工具，有截止和原Child退出确认。超时、输出超限或guard失败时仅回收本次直接子进程，回收未确认单独报告未知，不推断系统挂载已消失。
仅接受唯一原镜像路径、当前UID、未加密、非writeable的系统条目；固定挂载点必须唯一，设备标识严格解析，重复或歧义拒绝。持有挂载目录描述符并用fstatfs核对只读标志、实际设备和挂载根，再比较当前路径的设备/inode；子目录或另一个同名挂载不能替代挂载根。重复观察比较原目录和系统报告，不从观察结果获得卸载权限。
生产挂载通过下述m2先持久保存一次意图，监管attach及父死亡，独立核对系统结果并绑定原挂载凭据；设备名会重用，不能凭旧/dev/disk编号自动卸载。首次挂载、只读恢复、释放和Source接合由各自模块组合。

系统attach即使只读也可能更新DMG的校验缓存属性及ctime；当前观察接收挂载后重新完整认证的Image，不把它解释为挂载前后的持久凭据迁移。正式挂载须独立绑定原文件身份/字节/quarantine与允许的元数据变化，不能放宽既有Image::revalidate。Observation持有目录句柄，正常卸载前须释放本次所有读句柄；busy不升级为强制卸载。


## 镜像挂载元数据转换

image/transition消费原Image并建立挂载前描述符快照；完成转换时重新全量认证固定字节和同一dev/inode/长度/mtime，比较UID/GID/模式/链接数/flags/出生时间/ACL及全部可枚举扩展属性。唯一可新增或改值的属性为有界com.apple.diskimages.recentcksum；既有该属性不能消失，quarantine及其他属性必须原样保留。ctime可以更新，atime不作为读取不变条件；旧Image的严格重核不放宽。
转换前后均检查deadline和上层guard；verify_before在实际挂载效果前重新核验原Image/保护信息/全属性完全未变，不刷新基线，成功完成转换才返回新stamp的Image。对象不可反序列化，失败不写回、修复或清除属性；转换仅证明允许的最终差异，不证明attach曾执行、不授予挂载/卸载权威。m2一次挂载意图在效果前持有此快照并监管执行，不能用任意新Image替代转换。


## 磁盘附加身份

已有挂载观察不能只绑定/dev/disk编号。image/mounted/media通过固定ioreg IOMedia原始plist报告与同一安全Rust解析器，选择唯一整盘和卷对象，要求严格BSD名称、只读属性及整盘到卷的祖先关系。记录启动会话UUID和两者IORegistryEntryID，复核时必须全部相同；机器报告有输出/节点/递归预算，缺失、重复或歧义拒绝。
IOKit SDK的IORegistryEntryGetRegistryEntryID契约说明标识可跨任务识别注册表对象、仅在重启前有效，因此必须组合启动会话。它标识磁盘附加对象，不独自证明文件系统未曾重新mount，也不授予attach/detach权威；仍须结合固定Image、当前系统映射和原目录身份。m2持久挂载日志绑定此身份及原操作所有权，不从孤立设备名或观察结果恢复卸载权。先复核对象、再按BSD编号调用卸载，并不等同效果与原对象原子绑定；生产释放使用下述原始磁盘FD绑定效果边界。


## 一次性挂载日志

image/journal以独立qq-mount.lock序列化挂载事务，按请求保存m2签名记录，版本1按原缓存文件身份选择私有active目录，版本2加入请求身份形成独立目录。仅接受qq-images中的已认证私有镜像；同镜像的未释放记录阻止其他请求并发附加。目录只新建、不接管；原状态锁、私有父目录及原镜像身份在效果边界重核。Pending必须先于调用者提供的挂载操作持久发布；Pending/Unknown在恢复时只返回未知，不重放操作。
成功路径消费MountTransition，认证实际只读挂载及原附加对象，保存镜像完成指纹和挂载回执后发布Complete。恢复重新核对签名、输入、父目录、完整镜像指纹、媒体对象和实际挂载根；Ready持有原挂载目录句柄供后续Source读取，不授予卸载或QQ运行权限。长文件检查不占用状态事务，前后复核原状态锁身份与记录。
此日志负责一次调度和只读恢复；生产attach及释放监管见下节。未知效果保留原目录与挂载，不关闭未知设备；已认证释放的版本2目录按下述清理协议处理。不把调用者回调成功当作挂载已通过验证。

日志输入不含允许挂载更新的ctime，但完成指纹绑定完整stamp、保护信息与全部属性。请求索引最多扫描4096项；同路径或同dev/inode的未释放记录均阻止另一请求，索引损坏拒绝而不跳过。Ready只持有读取句柄，必须释放全部读取者后才可由后续独立协议考虑卸载；释放须显式进入下述持久协议。

## 生产只读 attach 监管

image/attach通过m2一次意图调用独立私有工作进程，工作进程持有原镜像和空挂载目录描述符，固定执行系统hdiutil attach -readonly -nobrowse -noautoopen -mountpoint，不接受调用者选择工具或附加参数。父端在派发许可字节前再次检查原意图guard；stdin作为许可/存活管道，EOF、额外输入、截止均取消。stdout/stderr仅传递原描述符，不输出业务内容或凭据。
父端不终止监管进程，而以关闭管道请求它回收自己的直接hdiutil子进程；监管进程独立截止，退出码只证明直接命令已结束，不证明系统磁盘辅助服务结束或挂载已撤销。挂载失败一律保留Pending/Unknown及现场，不自动detach、不强制卸载。只有m2后续独立镜像转换与系统观察成功，才发布Complete。

私有工作进程启动后先以有界非阻塞读取等待精确许可字节1，未获许可不执行系统挂载；错误字节、后续额外输入、EOF和截止均拒绝。镜像/挂载根须与继承描述符同身份，根为空且私有，描述符副本设置CLOEXEC。系统不存在既有镜像/挂载点的观察后再次检查原身份；父端轮询原guard，失锁通过管道取消，不把直接命令退出推导为挂载已清理。

## 原始磁盘 FD 与释放内核

DiskArbitration对象名不能单独防止设备重用：固定Apple源码中DADiskCreateFromIOMedia读取BSD Name后调用DADiskCreateFromBSDName，服务端按字符串查找当前磁盘。释放路线改用原只读raw设备FD：在已核验挂载根仍被持有时读取IOMedia的BSD Major/Minor，打开精确整盘与卷字符设备，比较fstat设备号，再复核原媒体代次和实际挂载。普通打开FD保留原minor槽位；只使用SDK DKIOCEJECT，不调用format或force，不在eject失败后复用该FD。
生产释放流程先持久保存释放意图，保持整盘/卷FD，在释放所有挂载根读取者后正常卸载卷，确认卸载并关闭卷FD，再消耗整盘FD执行一次eject。内核EBUSY保留未知现场，不升级force；成功返回不单独证明系统状态，须观察原附加对象及镜像条目消失后记录完成。FD身份及单次效果由下述m2持久释放、独立监管、只读未知恢复和目录清理协议组合。

## 持久释放意图

m2记录由已认证Complete进入绑定释放请求的Release(complete=false)，持有原qq-mount锁、状态锁身份、索引/缓存/active/父目录身份。先核对原镜像完成指纹与挂载回执，在挂载根仍被固定时建立原raw FD，然后持久保存Release，释放本次所有挂载根读取者，才允许释放回调执行。调用者须先释放Source及其他Ready；仍有读取者时普通卸载/busy不得升级force。
相同释放请求只做恢复，不重放卸载/eject；不同释放请求拒绝。回调返回码不是完成证明，原镜像指纹及系统镜像/挂载点映射、原启动会话IOMedia整盘/卷对象均确认消失后才记录Release(complete=true)。进程中断或失败保留未完成Release；恢复仅在独立消失观察通过后补完成记录，不重新派发效果。旧挂载请求返回Released而非重新attach。释放本身保留active目录与原记录，版本2可继续下述清理协议；不删除未知现场。

已认证Complete若在释放请求前即确认原镜像映射与原媒体对象全部消失，可在保存Release意图后只读记完成，不调用效果回调；这覆盖外部正常卸载或重启后的已消失状态，不能跳过原镜像指纹/原记录核验。已记录完成的相同释放请求返回历史Released，不重新取得读取或效果能力；后续新请求须等待下述清理完成，不能凭历史Released直接放行。

## 释放效果监管

生产释放入口在m2新释放回调内消费Devices，将原整盘/卷FD交给独立工作进程，不根据设备名重新打开。传递有界媒体/设备号元数据并核对原boot/IOMedia代次、当前主次设备号和继承只读字符设备身份；元数据本身不授予效果权限。worker复制继承FD到CLOEXEC句柄后，将stdout/stderr替换为null，避免遗留卷FD阻止eject或泄露给diskutil。
父端复核原意图guard后才发送许可字节，随后以原截止和guard监管；worker沿用EOF/多余输入取消协议，固定执行diskutil普通unmount，成功后消费原FD执行一次SDK eject。不使用force、不重开编号、不重试；父端取消关闭管道，不杀监管者。直接命令退出和eject接受均不能替代m2独立消失核验；内核调用未能及时返回时保留未知结果，不承诺强制回收内核或磁盘服务。

## 挂载目录清理与后续请求

新m2记录版本2在attach前绑定原空挂载目录device/inode，以输入及请求共同派生独立active父目录。版本1仍按旧输入路径恢复，不迁移、不从当前空目录补造原身份；缺少原mount身份时拒绝自动清理。签名记录和请求索引保留，旧请求永不重放。
仅Release(complete=true)可进入Cleanup；先认证原镜像完整指纹、原媒体/系统映射消失、原父目录和空mount身份，再持久保存清理意图。使用描述符相对rmdir，仅删除固定mount及其空父目录，不递归、不删除缓存或其他内容。Mount、Parent、Complete三个阶段在删除及父目录fsync之后保存；中断在同阶段重新观察，允许缺失项，但存在项必须仍为原身份。未知挂载/未完成释放不清理，额外内容/身份变化/新挂载拒绝。
仅Cleanup Complete记录放行同镜像新请求；新请求使用自己的新目录。旧attach/release/cleanup请求返回历史状态，不接触新挂载；清理过程中原锁与签名记录在效果边界重核。系统观察和rmdir不是原子事务，不声称抵御恶意同用户竞态或证明断电行为。

## 正式 CLI 启动编排

macOS start在运行平台模块内组合所选macOS目录版本安装、官方Source、n2预约、l2/r2准备、d2装配和s1一次启动；app只负责CLI选择及现有响应契约。首次未验证组合沿用--new --verify-compatibility显式试运行，不将原型证据冒充生产CLI兼容性正例。已完成装配恢复读取原d2，不重新复制或签名；挂载使用m2释放/清理协议，不持有Source启动QQ。
启动使用外部sandbox、全新环境和字面参数；数据写入仅受控运行目录及认证的同实例 profile，禁止读取普通用户目录及执行专用应用之外的任意程序，原系统安装不作启动目标。s1先于创建进程，复核d2/i1和原状态锁后只派发一次；保存j2原进程身份，随后状态为login_required，不自动登录或确认账号。失败保留原记录，不换请求重发。
j2生产记录绑定原启动目标及实例端点；HTTP/WS继续以原进程的同socket归属检查后发出认证字节。登录前控制材料从认证r2只读恢复，允许运行后的配置内容变化，但不更换原随机凭据或目录身份。首次端点出现由显式status观察，不从任意端口或PID接管会话。合成运行验证和实际固定QQ组合分别验收。

启动编排不递归持有生命周期锁：选择阶段记录原锁身份并释放，n2/l2/r2/d2各用自己的原生命周期事务；准备完成后重新获取并比较最初锁身份，再进入s1。派发前在s1原事务中重新认证d2/i1、l2目录、r2原秘密及精确配置和已发布适配器，准备后配置变化必须拒绝。只读prepare_instance供相同生产准备流程的独立宿主验收，返回不可序列化的观察，不授予写入或启动权限。
macOS napcat start默认900秒、最大1800秒，原普通查询/流及Windows范围保持不变；Clap、输入标量和Schema共享范围。首次组合尚无生产在线验证时使用原显式试运行开关，不能从原型或完成复制推导supported。n2离线请求查询不转换为n1；Windows旧创建记录仍可只读查询，但不能作为macOS启动权威。
装配错误在d2保存Unknown后保留原错误类型及固定阶段说明，不以通用未知错误掩盖实际失败。

## macOS只读诊断

NativeProbe的managed_platform报告位于app/diagnostics/macos，通用doctor保留产物/许可注记/API采集；系统安装发现单列doctor/discovery。macOS无显式版本和选中实例时默认v4.18.41，不读取Windows k1作为平台兼容权威。原生arm64宿主可直接确认nativeArch为aarch64，osBuild仍为null，不将架构或Darwin主版本冒充产品构建。
诊断新增typed managed_qq检查：required表示当前没有认证完成的部署，prepared表示原d2完整副本和i1安装核验通过，unavailable携带原始固定错误码；另列固定QQ版本/构建及官网DMG摘要。缺少部署记录时不访问签名密钥；已有记录只经References::existing读取，不生成权威。诊断不下载、挂载、装配、签名或启动。静态prepared和API在线都不能生成兼容正例；没有原启动在线验证器发布的匹配k2时兼容仍为unknown且canStart=false，显式试运行入口继续独立使用。

## macOS启动兼容证据

macOS使用独立k2软件组合记录，保留Windows Key/k1字节与读取规则。组合绑定Darwin内核release/version及只读SystemVersion.plist内容摘要（不提取、序列化或输出nodename）、原生arm64、固定Framework版本/提交/原始ZIP、官网QQ完整来源及角色策略摘要，以及按固定占位路径渲染的适配器/环境/参数、固定端口/合成秘密下的配置模板及外部sandbox策略摘要；占位路径仅用于比较软件行为，不用于实际路径授权。
s1启动输入除原d2、运行计划、sandbox和原始配置摘要外，还绑定该组合、完整字面参数/环境及工作目录。新j2同时保存可选软件键并关联原s1 Target；旧j2缺该键或键不同仍可只读恢复，但在线收集器不套用当前模板、不访问其API生成当前正例。只有声明键匹配且重建完整Target也相等才进入发布。既有缺少这些输入的启动记录不能被重算为新组合的正向证据。运行后由原n2计划、d2副本、r2原随机材料及已发布适配器重新构建输入，再与认证j2/s1逐字段匹配。
仅显式start复用ready实例时可调用在线验证器：持原生命周期锁，前后认证相同运行/部署及原进程，通过受检连接两次确认相同在线账号和版本，在原状态事务中复核后发布k2。status/doctor/普通业务读取不隐式生成兼容记录。无记录、旧记录、配置权威缺失或输入不匹配保持unknown；新实例只有匹配k2或显式试运行才可进入准备。新的系统、入口/环境/参数/配置/隔离策略、QQ或Framework组合不继承旧正例。

macOS实际子进程环境在适配器的全新环境上调用原Bundle.visit_environment，传入原r2的NAPCAT_WEBUI_JWT_SECRET_KEY；启动摘要使用同一个组合函数，不继承宿主值，不使用会被上游记录的NAPCAT_WEBUI_SECRET_KEY。软件组合只渲染固定占位秘密，不生成或序列化真实凭据；同时绑定package.main的固定相对入口表示，入口解析策略变化不继承旧k2。k2仅表示该原Framework启动与在线身份范围的证据，不替代GUI、事件、媒体和生命周期分项验收。

## macOS停止与完成证明

新启动的j2声明继承到直接后代的固定执行限制：仅可执行受管QQ.app内部代码；声明与实际sandbox摘要和s1 Target绑定。旧记录缺此声明不推断整树范围。停止使用独立p2意图和原生命周期/状态身份，不解释Windows p1/Job。原j2/s1允许为清理认证Pending/Unknown启动产生的原根，但不因此开放API或重放启动。
只向原根及在原父身份前后核对过的直接后代授予停止权威；暂停/终止各自先保存原audit token和一次意图，不发killpg或仅PID信号，不恢复用户已有暂停。冻结已拥有成员后收敛成员集合，已脱离而不能证明来源的候选不接管。失败保留原意图；恢复只观察原代次和已保存阶段，不重发未知信号。
完成证明要求所有已拥有成员结束，且在原部署的受检可执行文件集合上确认无可能仍属该运行的活进程。只读扫描限定当前UID，比较前后活进程集合/出生代次，匹配进程再保留原生观察；不输出或持久化无关进程元数据。固定继承执行限制和稳定出生集合共同约束fork/exec漏检；动态空列表本身不授予完整停止结论。跨启动会话可证明旧进程全部消失，不向新会话PID发信号。陌生/脱离候选或观察不确定均不标记stopped。先持久完成凭据再确认注册表；从未派发的准备态可在无s1认领时无信号完成。

停止实现位于runtime/macos/stop：journal只处理p2认证和形状校验，members只处理成员发现、冻结和原代次终止，session显式组合原生命周期锁、原状态锁身份、完整实例/安装快照、j2来源和d2完成回执。p2保存原d2记录摘要与成员父索引，最多256个成员；可执行文件集最多512项，当前UID枚举最多4096个PID，p2私有载荷上限1MiB。初次和完成前读取完整部署内容，效果间只用原清单stamp及原签名记录作身份守卫，不恢复写能力。原生信号返回成功后仍须独立观察暂停/退出；Pending的信号不因新CLI调用而重新派发。

新j2除固定执行声明外，还保存原根audit token；实际启动摘要升级为v3并绑定声明，兼容在线收集器只接纳同时有原声明和audit代次的记录。旧j2继续支持原只读服务恢复，不补造声明、audit token或当前兼容证据，缺失原停止权威时不自动终止。完成p2先于注册表stopped确认；确认中断只复用原完成证明，不要求原副本仍存在，不重复信号。失败可能保留已暂停的原成员，不自动SIGCONT用户已有暂停。

枚举不把任意观察失败当作进程消失。PROC_PIDTBSDINFO明确ESRCH或终止状态可排除已退出槽位；proc_pidpath的ENOENT仅在所有受检文件仍具有原路径、单链接和stamp时排除该映像属于此文件集，不表示该无关进程已退出。权限错误、截断、活出生集合变化或不在清单中的应用映像均拒绝完成；瞬态集合/退出竞争仅在同一截止中重做只读观察。固定应用不可写和继承执行限制是该排除的前提；其证明范围为直接继承沙箱的进程后代，不扩展为系统代理服务已关闭或抵御恶意同用户竞态。

## 实例登记移除

macOS复用通用remove/x1登记事务；x1只描述逻辑登记移除，不包含Job或原生停止能力。原注册表的完整实例与安装必须匹配已完成p2，plain stopped、Pending p2或Windows p1不能授予移除权威。先保存x1 Pending，再退役有原n2证明且精确匹配instance/run的d1活动指针，原子移除登记，最后保存完成x1。保持生命周期和原状态事务到最终回执，效果边界重核锁；已提交移除可在旧安装不存在时恢复。重复完成不操作之后创建的默认指针。macOS后继代次须由原g2来源链识别，不能仅按稳定ID推断来源。

默认指针只通过原父目录FD和仍匹配的单链接普通文件FD删除，并同步目录和设备；不递归处理运行数据。原签名记录、创建键、QQ副本、凭据、安装和请求历史保留。原移除索引的存在只用作防复活的否定证据：旧Reserved创建及后继登记也不得越过它重新登记，不将存在路径当作删除或停止授权。

## 同安装重启与停止后启动

macOS普通start选中stopped实例时复用restart用例。重启先认证固定i1安装、当前软件组合的正向k2、固定官网镜像可用性，并对原活运行重建j2/s1输入；失败不得先停止原窗口。停止与后继认领共持同一生命周期锁，并在效果边界重核原状态锁及完整实例/安装。preparing/prepared仅续接原n2请求；starting/unknown不重放派发。

代次算法与Windows共用，macOS使用独立g2：schema1保留同安装记录，schema2绑定源/目标安装及最近不同安装的回滚引用；g1/v1不能作为macOS权威。原完整p2完成证明先于g2认领；认领先于来源索引及n2登记。新run保留instanceId，清空账号/进程，使用新端点计划及独立r2凭据，保留原运行材料。源快照重复执行恢复同一后继，不再停止原运行；原来源缺失、原移除索引存在或登记回退均拒绝补造。

释放生命周期锁后只用认证n2内部请求续接正常准备/派发，不能凭稳定instanceId选取后来的运行。默认创建指针通过g2根来源识别准备中的后继及移除退役。restart总预算900秒；start仍可通过既有timeout指定1至1800秒。正式QQ、Keychain及在线账号验收仍独立进行。同安装重启和跨安装维护分别验收。

## 固定目录的维护命令

macOS目录默认v4.18.41，另接受v4.18.38的独立准备/试运行。update先在凭据/停止前拒绝平台目录外版本，并验证固定摘要及完整i1；目标为同一安装时，启动用例在原操作锁下比较完整实例和安装快照。ready/login_required沿原服务复用，preparing/prepared续接原n2，stopped消费原p2/g2建立后继；不因同版本update重启ready窗口，不复制Windows版本维护日志。

update的显式verify-compatibility只放行缺少k2的完整组合准备/启动；损坏证据仍拒绝，原安装、来源、凭据、装配和一次派发门禁保留，试验不直接发布supported。同安装stopped试验只建立该安装后继；跨安装试验由维护流程建立目标后继。普通stopped start及restart的后继准备仍要求原兼容证据。macOS update复用StartDeadlineArgs，默认900秒、范围1至1800秒；Windows保持原DeadlineArgs。无认证上一安装的macOS实例上rollback返回ROLLBACK_UNAVAILABLE，不读取Windows回滚历史。跨安装维护机制见下节，真实版本切换仍独立验收。

## 多发布版本的准备与证据隔离

macos/release集中定义可审计平台资产范围，默认仍为v4.18.41，另纳入经固定ZIP/元数据/三个arm64成员核验的v4.18.38；范围声明不产生supported结论。准备、材料、配置、装配来源、实际启动摘要、原运行重构、重启预检和doctor均从已选安装/原n2计划解析此目录，不再将其它版本按默认v4.18.41解释。未显式指定版本的已有实例按其原安装恢复，新实例才使用默认；显式版本不符仍拒绝，不从start自动切换安装。

Key::for_version绑定各版固定提交/ZIP及规范配置描述；k2记录按完整键匹配，两版在线证据不互通。v4.18.41的字段、规范配置及输入域版本保持原字节，已有同组合证据可继续匹配；新增可审计版本本身仍为unknown。在线观察前后核对同版已认证安装及实际API版本，只为该版原派发写入k2。prepare_instance从已有n2恢复版本；显式prepare_version为相同生产准备流程提供版本输入，不授予派发权限。

两个版本各自使用精确在线证据，未知组合仅由显式试运行放行；准备和合成维护不等同真实QQ版本切换验收。

## 来源收尾的预算与恢复

start/source的qq-source.lock覆盖一次来源读取、准备回调及退役，与短生命周期事务分离；同请求的完成恢复也取得此锁，避免并行准备尚持有Source读句柄时提前卸载。进入来源阶段后，为释放/空目录清理预留剩余总预算的四分之一，最多60秒；获取/挂载/Source观察和回调使用更早的工作截止，收尾使用原命令总截止，不延长公开timeout。

操作错误与收尾错误分别处理，保留原错误码/结果状态；两者均失败时追加固定的收尾未解决提示，不用次要DEADLINE覆盖原部署未知。已有n2的准备恢复先检查其原来源请求的认证m2；完整清理墓碑可直接跳过，未退役记录仅用原私有缓存继续原release/cleanup，不下载替代镜像，不重新attach或重放未知派发。qq-source.lock只协调读者，不替代m2设备代次、原始描述符、安装/部署认证或生命周期锁。

## 跨安装更新与回滚

macos/maintenance按preflight、显式事务编排及正常n2恢复分责。停止前验证源/目标完整i1、平台固定摘要、目标当前软件键及官方镜像可用性，活源另重建原j2/s1输入。缺少目标正向k2仅显式update试运行可放行；rollback不猜测兼容。先保存独立v2原源/安装/目标/动作索引和Registry标记，再调用原p2停止；同一生命周期锁内以g2 schema2认领目标后继，原v2引用沿g2保留。状态锁身份和完整快照在效果边界重核；登记后原标记退役，新运行清空账号/进程并用独立材料。释放原锁后只通过认证n2内部键续接准备/派发。

共享version_intent、generation和回滚目标解析只复用数据协议，Windows仍读写v1/g1；macOS读取v2/g2。Registry可只读保存两种前缀，但外平台记录不授予运行权限。跨安装g2必须绑定原v2，旧schema1仍可读取；同安装新代继承最近不同安装的回滚引用，不继承维护动作。当前/待维护目标/回滚安装均受卸载保护；登记移除退役当前标记而保留历史。

普通start/restart拒绝未完成v2维护，返回原动作指引；后继准备态的自动启动直接使用g2内部键，不重用旧默认指针版本。源快照重复请求复用原后继；回滚准备快照恢复其原目标，不反复切换。已完成后重新选择当前ready代并显式rollback属于新维护，不承诺无请求键的新命令能推断人类重试意图。缺少原历史返回ROLLBACK_UNAVAILABLE，已知派生创建但来源丢失仍拒绝，不修复或重新认领。macOS rollback与update同用900秒默认、1至1800秒总预算；Windows保持原预算。


## 同一实例的 QQ 数据继承

新目录记录使用 l2 schema 2 / layout 3，在已有原子准备协议中绑定可选的 profile 引用；首次实例留空并拥有自身 profile-home。后继只从认证 g2 的原实例/安装快照取得已完成 p2 的来源，再读取其完整 l2。来源已有 profile 引用时直接继承最初所有者的签名 l2 引用，避免递归目录链；否则绑定来源自己的 l2。引用只能指向同实例、不同运行的完整且无上级 profile 的记录。没有有效原目录身份、原引用或停止证明时拒绝，不扫描、移动、复制或修复未知用户数据。

l2 schema 1 / layout 2 继续按自身 profile-home 只读恢复，原序列化和指纹保持；旧记录不能被解释为继承了另一运行的数据。新版恢复同时检查当前运行目录和原 profile 所有者目录的 inode、属主、私有权限、ACL 与无链接路径；目录内容可由 QQ 正常更新，不把其内容摘要当成不可变配置。profile 绑定进入 l2 指纹，继续由 r2/d2 和完整启动输入绑定，账号观察和 API/control/JWT 凭据仍各次运行独立，不能从旧登录缓存直接确认新运行账号。

运行准备、装配、原运行重建和最终派发共同使用认证 l2 的 profile-home。Electron 数据路径及 HOME/CFFIXED_USER_HOME 指向该目录，temp/logs/config/NapCat 工作目录仍属于本次运行。sandbox 先拒绝读取整个私有状态根，再仅开放本次运行、所选Framework及该profile子树，profile允许读写，继续限制执行到本次专用应用；旧应用、旧运行凭据和普通 QQ 数据不因此获得访问范围。JavaScript 仅检查 profile-home 与本次运行同属受控 runtimes 父目录，跨实例授权由 Rust 的签名来源链与停止证明核对。

此机制保留同实例 QQ 数据供 QQ 自身恢复登录；QQ 若仍要求确认或验证，继续返回真实登录阶段。不得跳过 QQ 认证，也不承诺保留登录态可替代重新确认实际账号。源码、离线目录继承及真实免扫码行为分别验收。

## macOS 出站附件访问

消息上传仍使用messaging/attachments的不可变影本、全局配额、a1所有权和原请求意图，不另建上传缓存或绕过未知结果保护。macOS生产写入按原实例/运行的runtime_store::identity生成prepare-<runtime>-随机后缀目录；Windows沿用原命名和流程，历史影本仍按原签名和清单读取/清理。

QQ沙箱获得当前运行及同实例已认证、完整停止的祖先运行前缀下附件影本的只读访问。QQ可能保留发送时的原文件路径，因此继承QQ数据时也须保留对这些原影本的读取能力；不开放整个attachments目录，不授予写入、其它实例、未证明的运行、相似前缀或链接目标的访问。start/sandbox/history从原g2来源链和每项完整p2证明派生祖先范围，拒绝跨实例、重复代次、缺失证明及截止耗尽；不扫描附件、不新建缓存或凭据。祖先链对本次运行固定，新增发送或清理附件不改变原启动输入。

路径先按正则字面量转义，再序列化为Scheme字符串；当前及祖先前缀以同一受检正则组合，由软件策略标识及完整启动输入绑定。不能原地替换既有QQ进程的沙箱，规则更新后须停止原运行并重新验证实际软件组合，不从旧k2继承支持结论。读取接口仍验证当前实际账号、原消息和媒体引用；停止证明及影本读范围不产生跨账号CLI调用权威。

影本命名不授予写意图或清理权限；发送前仍须验证原账号、实例快照、原请求和a1字节摘要。上传错误或应答不明确时仍保留unknown及原影本，不因权限修复自动重发。
