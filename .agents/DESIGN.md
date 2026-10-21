# qq-cli 工程设计索引

CHECK-065：Windows质量、PR Semifold状态及main Semifold CD已落地；固定工具链、离线门禁、私有包GitHub Release资产与workspace锁同步明确。320项Rust、发布合成/实际10文件ZIP及解包离线运行、actionlint、240文件源码包通过；README/贡献指南及GOV/设计已同步，patch:chore登记。TASK-005本地完成、远端待部署验证，不创建远端或触发发布，不提升QQ业务/稳定性验收。

CHECK-064：内部架构和完整交付计划已从docs迁入.agents，文档编号保留，README/贡献入口及Cargo白名单同步；237文件源码包、公开本地链接和diff检查通过，patch:chore登记。TASK-005接下来实现Windows质量及Semifold CD，不执行推送/发布。

自动化与上下文整理已按用户2026-10-22授权开始（RFC-0007/SPEC-005/TASK-005）：内部DESIGN-002/PLAN-001迁入.agents并从源码包排除；后续Windows质量门禁及Semifold CI/CD独立验收。不执行本地版本消费、远端推送/发布或真实QQ操作，业务验收状态不扩大。

CHECK-063：角色旧值已定位为NapCat v4.18.38成员列表刷新时序问题：no_cache触发后台刷新却优先返回已有Map。固定源码方法的合成Promise实验复现，实际安装bundle一致，后续原实例四种列表/单成员查询均恢复member。帮助/README/发现Skill明确role为可能滞后的快照，保持数据原样且不自动重试写入；23项离线帮助/Skill/契约测试及release核对通过，patch:chore登记。两项本地排查已完成；上游缓存修复跟踪保留，稳定性暂缓。

CHECK-062：已捕获notify/input_status的group_id=0触发本地BACKEND_PROTOCOL，修复为notify无群哨兵；会话限定时过滤、无过滤时unsupported，真实群通知/畸形ID仍严格。新增2项回归后事件专项15项通过，发布CLI原群只读90秒正常截止，fmt/Clippy/release/结构与契约门禁通过；已登记patch:fix。CHECK-061角色缓存问题接着记录固定上游源码和只读证据，稳定性暂缓。

CHECK-062排查进行中：受检只读捕获notify/input_status的group_id=0导致BACKEND_PROTOCOL，已定位本地通知路由把无群哨兵按群号拒绝。SPEC-004明确notify零群哨兵的过滤/unsupported语义，接下来定向修复、离线及真实只读复核；角色缓存问题随后核对，不提前勾选完成。

CHECK-061：群撤回两端各一次，按各账号局部messageId与原消息匹配；目标账号group_admin/set及unset各一次，取消已回执/通知确认，自身列表恢复member。群主侧角色查询仍旧admin，直接NapCat含no_cache亦相同，不能用缓存作实时权限结论。四个通知监听均正常截止，13项事件离线测试通过。CHECK-060单次BACKEND_PROTOCOL在两次只读观察未复现、原触发帧缺失，仍待定位；不声明已修复。入退群/好友新增未授权不执行，稳定性暂缓。

CHECK-060：JPEG、双帧GIF及中文名二进制文件经实验群各发送一次，对端各接收一次，get/download成功且字节及SHA-256与原件一致；GIF保留两帧。两个实例media prune正常返回零删除并保留7项。附件功能通过；一侧事件流随后BACKEND_PROTOCOL提前结束，单列排查，不能称全部监听通过。下一步撤回/管理员通知及该普通事件解析问题；稳定性暂缓，业务68/100保持。

CHECK-059源码补核：固定450fb042的NapCat公开ActionName无建群API，但QQ内部服务声明createGroup/createGroupWithMembers/createGroupV2，参数仍unknown；内部声明不等于可从CLI调用的接口。真实附件/通知补验仍待操作范围确认，稳定性暂缓。

用户2026-10-21收窄登录验收：登录方式由官方QQ与用户负责，不逐一测试扫码/密码/已有账号点击；CLI只验收待登录观察、实际身份确认和账号变化隔离。下一步附件及其它通知正常功能验收；真实账号/目标/内容按本轮明确范围执行，稳定性继续暂缓。CHECK-059记录附件15项离线专项通过及固定API无建群入口的核对；真实附件/通知补验尚未执行，业务68/100不变。

契约整改已完成（CHECK-058）：实际Serialize/JsonSchema与Clap类型同源，条件/预算共享规则；无手写输出Schema、通用响应或Value桥。317项完整回归及8项旧游标/输入专项通过（318唯一用例），发布CLI核对34契约/45帮助/六Skill，Clippy、release、结构及239文件包通过。业务68/100保持。

CHECK-049：完成34叶命令/45帮助入口、Schema编译/参数映射及六类Skill覆盖核对，修正JSON与流scope、检查点选择、退出7说明及update参数表。输入为Clap派生加手工约束，输出仍手写；11命令responseSchema为通用data envelope，明确列为后续严格契约缺口，不宣称全部字段同源或交付门槛完成。CLI13+Skill3通过，业务68/100保持。

CHECK-048：补齐qq-cli本地w1签名检查点、稳定消息标识、指定会话有界补读及恢复摘要。双实例真实双向消息/接收方撤回通知通过，正常停止监听后补读1条、去重2条且保留gap，跨实例拒绝。29项唯一用例、clippy/release/架构/219文件包通过；仅1类notice真实、其它通知及重连仅合成，不宣称事件日志/恰好一次。业务清单68/100（原混合计数），稳定性暂缓；下一步帮助/Schema/Skill核对。

CHECK-047：用户确认新建两个实验实例并分别完成不同账号登录；两者同时ready/online且selected start均reused:true，未显式选择status返回四候选歧义。run/HTTP/WS/凭据键各异、共享同一固定38安装。plain processId均null不作归属权威，受检传输由原签名Job核对。无发信/撤回或其它QQ停止；事件恢复开发及帮助/Skill全契约仍待。

事件恢复设计（本轮确认，待实现/验收）：event watch --resume CHECKPOINT复用签名w1检查点，必须同时指定conversation；检查点绑定instance/account/run/conversation，仅保存起始时间及最近最多256个稳定消息标识，无正文，窗口最多24小时。ready/数据/summary携带可用检查点；先连接WS再作有界history补读，恢复与重连均输出coverage unknown的gap，消息带live/history来源，稳定标识去重。共享原limit/字节/总截止，补读最多一次有界history页，不做全量历史承诺；恢复不足仍报告gap/partial，身份或run/目标不匹配在领域读取前拒绝。

CHECK-046：安装/卸载正常功能通过：37缓存命中不读缺失来源、38被两登记引用拒绝、37无引用卸载仅移除681个目标文件且1167个其它文件不变；重复卸载明确NOT_FOUND，原固定包重新安装恢复两份安装及原实例登记。显式install是最终职责边界，取消隐式获取待办；不新增QQ运行，故障/网络异常矩阵仍暂缓。

安装职责确认（2026-10-20）：install显式获取/校验安装，start及update不隐式联网或新增安装；缺安装先返回安装指引，Agent在原授权范围内执行install后继续原操作，无需逐步询问用户。update保留结构化nextAction，目标缺失不停止原run。自动版本获取不再属于待实现项；运行配置/代次准备仍由start负责。

CHECK-045：原实验实例stop成功后remove首次removed/reused:false、重复reused:true，显式status为INSTANCE_NOT_FOUND。逐文件对照原1846项均保留且摘要不变，仅registry变化及3份注销凭证新增；两份安装/其它2实例登记不变。实验QQ已停止、另9个QQ进程与停止前完全一致。本轮版本/回滚/注销/Ctrl+C正常功能均通过，稳定性暂缓；自动版本获取等其它契约差距仍待。

CHECK-044：原实验会话真实event watch收到ready后，在专用隐藏控制台发送CTRL_C_EVENT；0.297秒退出130，summary.reason=cancelled、零gap/重复、stderr为空。随后同账号38 ready。17个QQ进程16个ID/创建时间不变，1个替换原因未定，不声称全部未变。无业务代码修复，正常取消通过；其它事件/恢复与稳定性不提升。下一步停止后注销。

CHECK-043：真实版本维护通过：固定v4.18.37导入、显式精确组合试验切换38→37、签名rollback恢复38、同目标update复用；两次status确认同账号在线，另9个QQ进程不变。补update试验/维护总截止和37身份API允许列表，60项定向通过。默认兼容拒绝/早期标签拒绝及debug预检超时保留，旧release被安全隔离非零风险结论；新release正常构建/实测。Ctrl+C/注销下一步，稳定性暂缓，67/100不变。

本轮四项正常功能验收完成；原实验实例已停止/注销，不隐式新建或恢复QQ。后续功能差距为其它事件/补读及Agent/交付对照，稳定性暂缓。

CHECK-042：日志完整保全、安装恢复ok；k1本地精确启动实证成功匹配原j1摘要/原r1/QQ与Job及前后在线身份。实际restart、stop→start经status恢复同账号ready，最终复用/歧义拒绝及另9个QQ进程保留通过；60项定向、clippy/release/217文件源码包通过，真实doctor读取不改签名状态。首次unknown和AUTH_REQUIRED检查点未重放。版本维护/注销/多账号新建及Ctrl+C仍待，稳定性暂缓，67/100保持。

CHECK-041：用户确认本轮对照，CHECK-040窗口确认关闭、CHECK-037新转发对照接受，CHECK-036本会话暂缓。列表/start复用通过，restart停止前因额外日志拒绝、QQ仍在线；后续CWD已隔离到同run，23项定向/clippy/release通过。旧日志未移动、doctor精确兼容证据仍缺；下一步保全日志/恢复安装和本地精确启动证据，再验收生命周期，67/100保持。

CHECK-040（2026-10-20）：私聊回复、群图片/文件/仅提及自身各一次accepted，历史唯一、回复目标/提及账号正确；群图片179及文件52字节下载与原件一致。已修复group显式/UUID被误按路径拒绝引用，10项定向及fmt/clippy/release/架构/213文件源码包通过。用户截图补证群图片显示、文件卡片名称/52B与自身提及；文件打开及私聊回复窗口仍待确认，不提升完整格式/稳定性或67/100混合计数。

CHECK-039（2026-10-20）：真实环境联系人/群/最近会话列表与续页、已知目标详情及名称/字段过滤正常，补齐普通发现路径证据。转发“可能漏当前账号发言”仅假说（旧缺项及新5条均与之相符，不是根因证明）；稳定性仍暂缓，原67/100不变。接着做已授权范围内私聊回复、群提及/附件，内容未确认前不发信。

CHECK-038：指定群详情、5+5成员续页、一次文本发送/一次引用回复及窗口对照、双向事件、3条历史/search/export功能通过，无业务代码修复。群提及/附件/群列表发现仍待；稳定性暂缓、原67/100保持。转发新样本5节点见CHECK-037，旧缺项CHECK-036仍未解决。

CHECK-037：新转发get展开5节点（含2个reply），两条后端路径均5条，CLI文字/顺序/数量一致；窗口5条尚未确认，CHECK-036旧样本缺项不撤销。纯上下文同步不创建changeset；完整转发未勾选，原67/100保持。当前继续指定测试群功能验收。

CHECK-036取代真实转发“待样本”状态：窗口4条，get_msg/get_forward_msg及CLI均仅1条；缺项发生在后端返回前，底层原因未确认。完整转发功能验收失败，expanded仅表示后端内容已展开；不修改NapCat、不拼接历史或假报完整。普通四项通过范围保留，稳定性暂缓、67/100不变。后续需验证可恢复完整节点的受支持上游入口或修复版本。

CHECK-035：同一私聊export 2+6恢复后8条与历史顺序/内容一致，完成后resume新增0，关键词/时间过滤单条及stdout无正文通过；消息8项合成测试通过。CHECK-032/033/034分别覆盖普通get、附件下载及search，四项普通流程无代码修复。真实合并转发待样本；群/全部格式及稳定性未验收，原67/100保持。

CHECK-034：同一私聊search扫描8条匹配5条，与本地历史筛选及1+4签名续页一致、零重复；一秒窗口单条及空匹配正常。无代码修改，下一步export；稳定性暂缓、原67/100不变。

CHECK-033：同一实验私聊合成图片/文件下载ok，179/52字节及SHA-256与发送原件一致，并经完整字节比较；现有配置正常路径可用，无业务代码修复。下一步search/export；群/全部格式及稳定性仍待，原67/100保持。

CHECK-032：同一实验私聊已有文本/图片/文件get功能通过，元数据及原回执ID一致；普通文本expand通过，真实合并转发待样本。无业务代码修改，下一步附件下载/search/export；稳定性暂缓、原67/100计数不变。

执行顺序调整（用户2026-10-19）：优先完成全部功能性实现与正常流程验收；稳定性专项（崩溃/故障注入、跨进程竞争、断线恢复压力及长期运行）暂缓，不删除既有保护、不把暂缓项勾选完成。功能顺序：消息get/附件下载/search/export → 联系人/群/最近会话与reply/提及/群收发 → 安装/实例/更新回滚正常流程及自动准备缺口 → 事件正常流程与Skill/Codex/OpenCode调用 → Schema/帮助/文档及功能总验收。已有兼容、身份、作用域、防重与未知结果保护继续执行。真实读取/发信/生命周期操作仍遵守具体授权，不由本排序扩大既往单次实验范围。67/100为原混合清单计数，不作为功能完成率；稳定性与完整交付门槛保持未验收。

CHECK-031：历史单锚点末端修复及11项定向、fmt/clippy/release通过；同一实验私聊5条与2+3续页一致且零重复，图片/文件各一次accepted、历史ID/类型吻合且用户确认窗口正常。仅增强既有子项证据，67/100保持；完整历史异常/媒体及Agent验收仍待。证据见EVIDENCE-001登记的history-boundary-media-send.md。

文档编号：DESIGN-001。状态：Active。更新：2026-10-19。

CHECK-022停止后start与后继准备恢复已通过有界离线验收：共享原准备/一次调度/保留用例，保留实例ID、原安装版本和签名权威，未知结果不重放；该阶段20项定向及完整271项回归、fmt/clippy/release/架构/199文件源码包通过。最新进度及重启接合见下段CHECK-023，真实QQ及完整故障矩阵仍待。

CHECK-023重启编排已通过有界离线验收：公开入口同锁预检先于停止、原p1/g1来源复用后继、后继准备恢复及未知不重放，23项定向及完整275项回归、fmt/clippy/release/架构/200文件源码包与Semifold检查通过。接合子项单列后业务65/98、运行环境25/34、待完成33；真实QQ和完整跨进程/故障矩阵仍待，未知结果先status/doctor，不自动重复restart。

停止/注销与凭据的历次失败、修复和有界验证见CHECK-020/021；保留历史失败，凭据协调缓解不等于已确定底层根因，不将子项验证扩大为完整生命周期结论。

CHECK-028公开rollback已接合，30项启动/停止定向及完整293项回归、clippy通过：只取原v1动作或签名回滚安装，锁内重核；准备恢复不反向切换，无历史/动作冲突/未知拒绝，旧准备快照不复活已停止后继。加强既有已安装目标维护子项，67/100保持；自动目标准备及完整故障/真实QQ仍待，不把命令接合当完整产品交付。

CHECK-029已在用户明确授权下通过CLI安装官方包，并修复目录虚拟化原凭据恢复、ASAR入口、Windows环境名及普通盘符ABI。最终新实例完成预登录保留，受检status为waiting_qrcode、账号未确认，控制进程有主窗口；两个早期失败记录保留。最终298项串行调度回归及fmt/clippy/release/架构/213文件源码包、Semifold通过；早期并行失败原因未定，记录保留。67/100不变，不将待扫码升级为账号/消息或完整兼容验收；后续登录及定向消息授权和结果见CHECK-030。

CHECK-030：用户登录后已用qq-cli验证一个授权私聊的实际身份、单条发送accepted/窗口一次、120秒双向事件及最近两条历史ID一致，监听正常截止且无gap/重复。较大页no_progress返回部分结果，连续历史穷尽仍未证明；群/媒体/恢复及完整Agent验收保持待办。增强既有子项真实证据，67/100不变，本轮未改业务代码；不扩展会话范围或后续发送授权。

CHECK-027停止前维护意图已通过有界离线验收：v1源索引→当前Registry标记→p1→g1签名引用，新run发布与标记退役同次提交；start/restart阻止旧版本启动，卸载保护目标，已知标记不补建密钥，明确注销退役标记而保留历史。完整290项及补充4项意图、fmt/clippy/release/架构/209文件源码包和Semifold通过。加强既有update子项，67/100、待完成33保持；自动目标准备、公开rollback及完整故障/真实QQ仍待。

CHECK-026已安装目标update已通过有界离线验收：同锁目标预检/原Job停止/版本代次/共享启动，同目标及准备恢复复用，未知拒绝；卸载同锁关闭内部目标删除窗口。26项启动/停止、2项锁内核及完整284项、fmt/clippy/release/架构/204文件源码包和Semifold通过，夹具自锁失败/修正记录保留。子项单列后业务67/100、运行环境27/36、待完成33；停止前持久意图后续见CHECK-027，自动目标准备、公开rollback与真实QQ仍待。

CHECK-025第二个固定Framework版本已通过静态来源/配置及离线验收：v4.18.37官方附件与实际包摘要匹配，入口/配置/只读控制依赖核对，默认v4.18.38不变。7项原生定向、完整280项、fmt/clippy/release/架构/202文件源码包和Semifold通过；受限环境2项归档失败记录保留。66/99、待完成33保持，目录扩展不代表公开update/rollback或实际兼容完成。

CHECK-024版本代次/回滚引用元数据已通过有界离线验收：g1兼容旧格式，分别绑定源/目标安装并缓存最近回滚目标；卸载保护回滚及已认领未发布目标。15项代次、3项默认启动独立复核和第二轮完整280项通过，fmt/clippy/release/架构/201文件源码包与Semifold通过。首轮2项默认启动超时/状态忙原因未定，保留失败记录。子项单列后业务66/99、运行环境26/35、待完成33；公开update/rollback及实际兼容仍待，第二个固定产物目录的后续审核见CHECK-025。

当前目标active。待登录status已接原签名控制能力、原管理秘密和受检HTTP，输出loginObservation但账号未确认；仅OneBot核对真实身份后确认就绪。3项独立CLI测试覆盖控制ready不提升、初始化/版本拒绝/最终账号确认、2FA/拒绝/并发换run及缺原材料不修补；完整231项离线回归及fmt/clippy/release/架构/174文件源码包通过，见CHECK-019。公开schema、runtime skill及用户文档同步，不扩大真实QQ操作授权。

完整业务/交付67/100，运行环境27/36；已安装目标update/rollback及停止前持久意图已接合，完整版本维护尚未闭环，未完成仍33项，数量不是工作量比例。质量整改8/8见CHECK-017；许可未知仅作信息见FACT-002/CHECK-018，精确兼容和归属门禁保留。下一步接自动目标准备与完整故障验收；真实首次启动、登录挑战、完整生命周期故障矩阵及整体验收仍待完成。

空实例status补修（CHECK-019）：not_started现明确identityVerified:false并纳入同一Schema；13项CLI定向、全目标clippy、fmt及源码包检查通过，不初始化状态或凭据。完整231项为此前基线，本修正不新增清单计数；下一步仍为实例生命周期。

以下为此前增量的历史状态；其中“当前/下一步”均为当时记录，最新进度以本页顶部及清单为准。
许可策略修正已离线验收，见CHECK-018：220项及fmt/clippy/release/架构/源码包检查通过；业务维持59/92，不提升真实运行或分发验收。

状态版本门禁增量：status与业务操作共用固定版本校验，不匹配时拒绝就绪输出和账号更新。原创受管CLI覆盖版本与账号同时变化，见CHECK-013；不新增完整交付勾选，下一步仍为托管生命周期与登录恢复。

start已就绪复用路径经原创签名Job服务验证：固定版本/在线账号核对，版本冲突及账号变化拒绝，--new不复用；无注册表改写或新增意图。见CHECK-013；完整新建/兼容门禁/登录恢复仍未完成，不勾选完整start任务。

实例创建原语新增独立4进程屏障证据：同键仅1个Fresh、同实例/run与1份认领，不生成运行材料/进程；不是公开CLI端到端。测试夹具改动及新增用例的fmt/clippy通过，最近全量仍为203项；整体58/91不变，完整准备/调度跨进程与公开新建仍待完成，见CHECK-013。

三端口租约及原生交接内核已验收：实例预留/固定调度共用，固定端口在Job分配前取得并持有到j1发布后、CreateProcess前；忙端口无Job/进程，部分租约失败释放。新增2项测试后完整复核203项及fmt/clippy通过；首轮event_schema临时STATE_IO根因未明并保留记录，见CHECK-013。不承诺SO_REUSE排他或实际listen原子交接，公开新建/端口全竞争仍待验收；拆分内核子项后58/91、运行环境18/27。

登录前生命周期缺口已有候选原型CHECK-013及固定源码/匹配摘要包核对CHECK-015：WebUI调用先于登录等待，OneBot后置，直接Node监听支持同进程架构推断；异步失败、端口漂移及实际QQ PID仍须验证。RFC-0003未冻结，真实兼容证据不提升，58/91不变。静态核对已完成；下一步接明确的固定端点/漂移失败处理和新建/端口交接，保留真实完整元组验收缺口。

实例创建预留内核已有3项原创线程/恢复/端口与1项丢签名权威保护证据：n1认领先于无账号登记，同键复用、不复活注销实例；当前完整201项离线回归及fmt/clippy通过。拆分后58/91、运行环境18/27；公开新建/零实例选择、实际端口交接及完整跨进程创建仍待验收，见CHECK-013。

首次服务发现内核和pending状态工厂已有原创父子进程证据：创建前签名预期映像/端点、一次固定零字节连接核对实际PID/映像/Job、先保留后绑定；重复恢复不连接，错端点/Job及旧缺输入日志拒绝。任务拆为已验收子项，当前58/91、运行环境18/27；真实QQ首次发现/完整新建/二维码仍未验收，见CHECK-013。

已绑定服务登录确认已有原创CLI证据：未绑定状态不访问计划API，版本不匹配不提升；匹配在线账号才在完整实例/安装快照复核后转ready，离线不提升，变化拒绝覆盖。6项传输及6项状态定向测试通过；首次服务发现/二维码/完整新建与恢复仍未完成，不勾选总任务，见CHECK-013。

最新完整回归为196项，fmt/全目标clippy通过，已覆盖锁内输入/材料及完整调度绑定三个增量；此前193项记录保留为历史。整体58/91不变，完整公开新建、真实原版兼容及加载绑定/登录恢复仍未验收，见CHECK-013。

完整调度绑定模块已接输入/材料同锁复核、三参数与白名单环境、版本化输入摘要及原生一次入口；新增1项原创布局/错误摘要拒绝及正确摘要原子创建证据，runtime_store共7项和原生调度2项通过。替身为临时编译的原创Rust三参数等待程序，只写自有标记及等待释放/15秒退出，不加载QQ/DLL/JS；不能作为真实原版验收。公开新建/最终加载绑定仍待接入，任务计数不变，见CHECK-013。

运行材料同锁只读复核已有新增1项原创证据：prepared/starting验证、配置/秘密/安装变化拒绝及真实start_intent内计数回调接合；runtime_store共6项及fmt/clippy通过，最近全量仍为193项。无真实进程，不勾选公开新建或运行后恢复，见CHECK-013；下一步接合完整启动调度门禁。

锁内输入复核接口已有新增1项原创证据：操作存续期持状态锁，安装目录/QQ内容/兼容证据变化拒绝后续操作；startup共5项定向测试和fmt/clippy通过，最近完整回归仍为193项，见CHECK-013。尚未接公开新建调度，不以此勾选完整生命周期或最终加载绑定。

启动输入只读编排已有4项原创定向证据及当前完整193项离线回归，fmt/clippy通过：授权/唯一安装/摘要与完整性、最多16个QQ候选及精确Framework兼容证据串联，失败不创建状态或访问账号API。Inputs不是启动许可；最终锁内文件绑定及完整启动仍待完成，见CHECK-013。完整任务计数保持58/91。

安装来源编排已接公开install：授权先于输入/联网/凭据，惰性本地导入与缓存复用、源错误/摘要拒绝及CLI无状态许可拒绝有原创证据；新增5项、当前完整189项离线回归及fmt/clippy通过。固定HTTPS下载仅规则核对，真实联网和完整安装不勾选，见CHECK-013。任务总数保持58/91，下一步补启动门禁与生命周期及下载故障验收。

## 产品方向与当前状态

面向个人 QQ 的本地 Agent 命令行接入，保留官方桌面窗口，与用户共用同一会话，不抢占输入焦点。
不依赖 LiteLoaderQQNT，不创建独立官方机器人身份。当前完成初始化、审计、有界实验与工程基础，核心模块及部分集合读取已有离线证据，完整功能未验收。
NapCat v4.18.38 的 Framework 包已有原生加载入口，优先评估原版，不必立即开发加载器或 fork。
CHECK-005、CHECK-006 已有实验账号登录、基础 API、消息历史对照和短时焦点证据；长期共存、原始编译来源与适用许可仍有缺口。
不依赖 LiteLoader 不等于不使用 Framework 的内部接入逻辑；也不等于无 Hook 或注入。
FACT-004 补充维护者曾推荐原生入口，旧 Once/OneKey 才是明确弃用对象；现行指南缺席原因未明。
两种模式均有 Bypass 调用，不能保证账号安全；如要求无下线或封禁风险，当前候选不满足要求。
原 M0 工程基础保留；已实现34命令解析/基础注册表、配置/私有状态、HTTP/身份门禁、集合续传、历史/搜索/导出与文本/发送内核、写意图模块、离线Skill及固定目录/解包与安装登记/卸载内核及分层预检，并补启动意图/进程/Job、原子根创建、后代保留、TCP归属与受检HTTP/WS，184项离线测试通过，见 CHECK-013、CHECK-014。
完整交付契约入口 PLAN-001、SPEC-004；Rust/Agent 决策 RFC-0002，实施 TASK-002。
当前已扩展到多账号、发现、私聊/群、历史、媒体、事件和 Agent 指引；基础5项已验收，扩展实现/交付59/92。
公开 UX 使用 skill 与 napcat，自动管理环境，单候选实例自动选择，多候选用 --instance；不公开 profile 或托管模式。
install/installations/uninstall 管理安装，start/instances/stop 管理实例；固定版本、引用保护及具体机制见 RFC-0003。
按语义/职责拆分见 RFC-0004：contact/group/conversation 各司其职，独立 reply/export，查询范围不隐式变化。
默认值、引用推导、紧凑输出与按需帮助降低成本，不以命令数量最少为目标。
当前用户授权进入持续开发，按依赖完成模块化实现和离线验收；最新要求每个独立验收部分同步适用changeset并本地提交，覆盖此前禁止暂存/提交约束。真实会话回归仍按明确授权边界，提交不代表完整交付。
QQ元数据、精确兼容键和doctor分层诊断已有CHECK-014；当前真实完整Framework证据为空，不自动提升为supported。
下一步补齐实际启动绑定/控制面、完整领域schema、历史恢复、附件/媒体/事件及宿主整体验收。
严格签名CLI路径及旧合成夹具迁移已验收：普通HTTP/WS回退已删除，status/联系人正向与账号/记录变化拒绝、发送/回复/文件/事件及离线回执均经原创Job服务验证，184项离线回归通过，见CHECK-013；完整启动、登录恢复和真实协议仍待验收。
共享客户端工厂接普通/事件/doctor，受管签名失败不降级、缓存回执离线及丢密钥不补建已验收，见RFC-0003、CHECK-013；旧合成登记已迁移，完整登录恢复/真实协议不勾选。
j1持久服务绑定内核已验收，见RFC-0003、CHECK-013：原流复核及t1成功先行，跨调用恢复逐项核对run/安装/端点/进程/Job；客户端工厂见严格CLI验收；完整登录恢复与真实协议仍待完成，不标记账号ready。
监听进程发现与Owned事件Source已定向验收，见RFC-0003、CHECK-013；仅该连接服务PID与可信Job交集、严格映像匹配及原流复核，不扩大为全局QQ发现。客户端工厂已严格接入；完整登录恢复与真实协议仍待验收。
原生调度按RFC-0003补j1日志：先保存随机Job再创建根进程，仅从签名完整根身份恢复；内核离线验收见CHECK-013，完整门禁接入仍待完成，不代替启动门禁。
保留事务按RFC-0003在现有状态锁中先发布t1签名pending再复制，一次/轮运行，未知不重做；该日志不替代远端当前状态或完整启动门禁。
受检WS按RFC-0003在共享握手路径前验证原socket、握手后复核Job/进程，不另建连接；CHECK-013有IPv4/IPv6原创服务证据，32KiB握手预算已同步SPEC-004，托管选择已接严格CLI；完整登录恢复与真实事件协议仍待验收。
受检HTTP按RFC-0003接入hyper连接级接口，CHECK-013已有独立服务证据：归属核对返回的同一socket、不由reqwest重连、普通/媒体解析共享；受检WS通道亦有证据，托管入口已接严格CLI；完整登录恢复与真实协议仍待验收。
依据RFC-0003实现已连接流的四元组/PID/Job观测，IPv4/IPv6原创回环定向证据见CHECK-013，不仅查监听端口；受检HTTP/WS通道使用同一流，完整CLI选择/账号关联仍待验证。
依据RFC-0003实现Job成员后代保留只读句柄，原创父子正反对照见CHECK-013：完整身份与同一目标句柄成员核对后复制非继承QUERY副本，不执行目标代码；一次性日志/恢复仍须接入，未知不自动重复，真实QQ兼容未验收。
原子创建内核见RFC-0003、CHECK-013：JOB_LIST在创建时入Job，HANDLE_LIST仅传QUERY副本及NUL设备，显式环境和参数，使用原始进程句柄；根进程自有子进程实验通过，尚未证明原版子树生命周期。
按RFC-0003实现随机命名Job查询内核，只有隔离原生模块可用有注释unsafe，其余代码deny；自有进程实验证明最后句柄关闭后成员仍活着但名称不可重开。仅持久化名称不足以跨CLI归属，须继续验证句柄保管、原子入Job及真实原版子树行为。
按RFC-0003补齐持句柄进程观测并有CHECK-013自有子进程证据：PID、创建时间、映像路径及零等待退出检查；不能代替加载摘要、父子/监听归属，不枚举接管已有QQ。启动run认领先于请求索引/starting，关闭prepared崩溃窗口的换键重放。
启动意图内核按RFC-0003实施并有CHECK-013定向证据：s1签名先落盘再调度，重复/中断不重放；仍须实际进程/监听与完整启动门禁验收。
私有运行材料准备事务已有CHECK-013证据：签名认领、整组秘密恢复、暂存前缀修复与prepared登记，不执行spawn；实际进程/监听绑定仍待完成。
无账号预设的托管配置生成内核已通过CHECK-013定向验证；固定默认配置复制与WebUI端口/日志风险见FACT-001；已接私有准备内核，未启动QQ。
图片来源定位、固定CDN HTTPS内核与身份守卫后发布已有CHECK-013合成证据；未知来源拒绝，托管配置/图片完整CLI及真实协议仍待验收。
独立HTTPS传输与图片编排已定向验收，使用原生证书/主机名验证和内存TLS假服务；不升级为真实QQ/CDN验收。
SPEC-004总截止贯穿已定向实现：准备与领域阶段不重新起算，可取消边界过期不进入下一阶段，证据CHECK-013；不宣称抢占同步系统调用。

## 文档注册表

| 编号 | 文件名 | 类型 | 责任 |
|---|---|---|---|
| README-001 | README.md | 介绍 | 产品目标与当前可用性 |
| CONTRIB-001 | CONTRIBUTING.md | 贡献指南 | 工程命令与协作入口 |
| GOV-001 | AGENTS.md | Active 约定 | 授权、事实、隐私与验证纪律 |
| DESIGN-001 | DESIGN.md | Active 索引 | 方向、阅读顺序与责任边界 |
| DESIGN-002 | .agents/architecture.md | 内部维护入口 | 源码职责、应用组合、原生边界与结构检查 |
| SPEC-001 | product-baseline.md | Active Spec | 已确定目标与非目标 |
| SPEC-002 | repository-bootstrap.md | Active Spec | Git、Semifold、上下文初始化 |
| SPEC-003 | framework-integration-plan.md | Active 验证计划 | 原版入口、生命周期及分阶段验收 |
| SPEC-004 | cli-product.md | Active 实施计划 | Agent CLI 完整产品范围、协议和验收 |
| SPEC-005 | specs/automation.md | Active 自动化契约 | Windows质量门禁、Semifold CD及交付边界 |
| PLAN-001 | .agents/specs/cli-design.md | 内部完整交付契约 | 命令/API 映射、配置、输出、发送和交付 |
| FACT-001 | napcat-architecture.md | 源码观察 | Shell、桌面会话接入与未验证边界 |
| FACT-002 | napcat-release-license.md | 固定静态观察 | 版本、许可、真实附件与接口 |
| FACT-003 | native-loader-provenance.md | 历史及静态观察 | 原生引入/维护、构建来源和版本证据矩阵 |
| FACT-004 | framework-history-safety.md | 历史及静态观察 | 文档修改、官方答复、Bypass 与账号安全边界 |
| EVIDENCE-001 | evidence-register.md | 证据索引 | 公开来源和验证范围 |
| DATA-001 | napcat-v4.18.38.json | 静态清单 | 固定提交与附件及关键成员摘要 |
| DATA-002 | native-loader-provenance.json | 追溯数据 | 主线观察边界、历史 blob、PE 元数据与支持证据分类 |
| CHECK-001 | bootstrap-verification.md | 验证记录 | 本次初始化结果 |
| CHECK-002 | framework-audit-verification.md | 静态验证记录 | 下载校验、源码匹配与未运行边界 |
| CHECK-003 | native-loader-trace-verification.md | 追溯验证记录 | 历史、打包、相邻仓库及公开报告的调查范围 |
| CHECK-004 | framework-history-verification.md | 静态验证记录 | 文档历史、弃用范围及两种模式安全结论复核 |
| CHECK-005 | framework-runtime-trial.md | 实验记录 | 原版启动预检及新实例尝试，不保证会话隔离 |
| CHECK-006 | framework-message-focus-trial.md | 实验记录 | 用户指定会话的有界收发及匿名前台焦点观察 |
| CHECK-007 | cli-plan-verification.md | 文档验证记录 | CLI 方案交叉核对及检查范围 |
| CHECK-008 | cli-m0-verification.md | 离线验证记录 | 工具链、命令骨架、版本迁移及离线测试 |
| CHECK-009 | group-history-boundary-verification.md | 只读实验记录 | 指定群历史时间边界和分页方向，不证明完整归档 |
| CHECK-010 | agent-cli-plan-verification.md | 设计验证记录 | 完整 Agent 产品契约、任务同步和离线检查 |
| CHECK-011 | instance-identity-design-verification.md | 设计及静态验证记录 | 命令 UX、安装/实例分离、自动选择和真实身份校验 |
| CHECK-013 | cli-core-verification.md | 开发离线验收 | 模块/CLI离线：读取、写意图、媒体、事件、安装和私有运行材料准备 |
| CHECK-014 | runtime-preflight-verification.md | 开发定向验收 | 安装元数据/架构/摘要、精确兼容键及doctor分层诊断 |
| CHECK-015 | prelogin-lifecycle-source.md | 固定源码观察 | 预登录WebUI、OneBot构造链与端口漂移及候选边界 |
| CHECK-016 | control-protocol-source.md | 固定协议观察 | 管理鉴权/登录阶段，非实际控制验收 |
| CHECK-017 | maintainability-refactor.md | 质量离线验收 | 职责重构、220项逐项对应、构建/打包及架构检查 |
| CHECK-018 | license-runtime-policy.md | 许可策略核对 | 固定原文、API/下载/启动边界及移除不确定许可硬阻断 |
| CHECK-019 | login-observation-verification.md | 登录观察增量验收 | 固定协议、受检控制传输与运行恢复分阶段记录 |
| CHECK-020 | runtime-maintenance.md | 维护机制及离线验收 | 原Job停止权限、持久证明与生命周期恢复 |
| CHECK-021 | credential-recovery.md | 有界离线验收 | 原生凭据跨进程协调、并发恢复及异常释放 |
| CHECK-022 | run-generation.md | 有界离线验收 | 稳定实例的新run、签名计划及停止后start恢复 |
| CHECK-023 | restart-orchestration.md | 有界离线验收 | 同锁预检/停止/后继、原run复用及未知拒绝，完整275项回归 |
| CHECK-024 | version-generation.md | 元数据有界离线验收 | 原/目标安装、回滚及未发布目标卸载保护；第二轮完整280项 |
| CHECK-025 | second-framework-version.md | 静态核对及离线验收 | v4.18.37独立包/入口摘要、配置与只读控制协议；完整280项，不继承实际兼容 |
| CHECK-026 | version-update-orchestration.md | 有界离线验收 | 已安装目标update、原Job切换/准备恢复及卸载同锁，完整284项 |
| CHECK-027 | durable-version-intent.md | 有界离线验收 | 停止前v1意图、Registry标记/g1原子交接及旧版本门禁，完整290项 |
| CHECK-028 | rollback-orchestration.md | 有界离线验收 | 原动作/回滚目标选择、准备恢复和停止后不复活，完整293项 |
| CHECK-029 | live-start-repair.md | 有界真实预登录与离线验收 | 原凭据恢复、ASAR/环境/路径修复，新QQ等待二维码，298项串行回归；无消息验收 |
| CHECK-030 | live-message-verification.md | 有界真实私聊验证 | 登录身份、单条发送、双向事件/最近历史ID一致；较大页no_progress仍有边界 |
| CHECK-031 | history-boundary-media-send.md | 有界修复与真实对照 | 包含锚点末端、无重复续传、图片/文件发送及窗口确认 |
| CHECK-032 | functional-message-read.md | 普通消息功能通过 | 文本/图片/文件get及普通文本expand；真实转发样本待补 |
| CHECK-033 | functional-media-download.md | 合成附件功能通过 | 私聊图片/文件下载与原输入逐字节一致 |
| CHECK-034 | functional-message-search.md | 范围搜索功能通过 | 文本匹配/条件恢复/时间窗口及空结果 |
| CHECK-035 | functional-message-export.md | 导出功能通过 | 2+6恢复、完成续传零新增及过滤导出 |
| CHECK-036 | forward-content-discrepancy.md | 真实完整性失败 | 窗口4条、后端/CLI仅1条；相关上游报告与语义限制 |
| CHECK-037 | second-forward-sample.md | 后端五节点对照通过 | 新样本两条后端路径与CLI一致，窗口计数待确认，旧缺项保留 |
| CHECK-038 | functional-group-messaging.md | 指定群普通功能通过 | 成员分页、文本/引用回复、双向事件、历史/search/export |
| CHECK-039 | functional-discovery.md | 真实环境发现通过 | 联系人/群/最近会话分页、已知详情及过滤范围 |
| CHECK-040 | functional-reply-mention-group-media.md | API功能与群文件修复通过 | 私聊回复/自我提及/群图片文件下载，窗口确认待回复 |
| CHECK-041 | lifecycle-working-directory.md | 实例预检与CWD修复 | 列表/在线复用通过，真实重启受旧日志/精确证据缺口阻断 |
| CHECK-042 | local-startup-evidence-lifecycle.md | 精确实证与正常生命周期通过 | 日志保全、签名本地观察、restart及stop/start恢复、其他QQ保留 |
| CHECK-012 | cli-ux-review.md | 设计核对记录 | 接口整体审视及语义职责修订、正常分页与新实例防重 |
| RFC-0000 | README.md | Active 流程 | 决策状态和接受条件 |
| RFC-0001 | 0001-session-loader.md | Draft RFC | 原生入口优先、自有加载器备选与不 fork 决策 |
| RFC-0004 | 0004-cli-ux-consolidation.md | Accepted RFC | 资源/任务职责拆分、明确查询范围、合理成本与防重复创建 |
| RFC-0003 | 0003-managed-runtime.md | Draft RFC | Agent 托管与登录/窗口冲突边界 |
| RFC-0002 | 0002-cli-api-client.md | Accepted RFC | Rust、Agent 产品契约与待验证设计 |
| RFC-0005 | 0005-maintainable-architecture.md | Accepted RFC | 质量前置整改、职责分组和应用/生命周期边界 |
| TODO-001 | TODO.md | Active 索引 | 实施清单入口 |
| TASK-001 | project.md | Active 清单 | 当前进度和后续验证差距 |
| TASK-002 | cli-product.md | Active 清单 | 基础已验收，扩展实现/交付 60/93 |
| TASK-003 | code-quality.md | 质量整改8/8完成 | CHECK-017；实现已提交c32680a，不计新增业务 |

## 候选责任边界

| 层 | 预期职责 | 当前状态 |
|---|---|---|
| CLI | 命令、配置、JSON、错误、账号门禁及发送记录 | 目标解析、配置/门禁已实现；完整 schema 和发送待实施 |
| 本地 API 适配 | 类型化 OneBot HTTP/WS、能力证据与结果校验 | 固定HTTP与部分集合适配已实现，完整用例待实施 |
| 运行管理 | Agent 托管原版环境、配置、启动与恢复；兼容外部实例 | 机制待验证，见 RFC-0003 |
| 上游 NapCat | QQ 内部能力与协议适配 | 本轮不修改，不随 CLI 打包 |

当前核心模块有合成验证，完整业务仍在开发；不新增真实 QQ 操作或变更安装。
2026-10-17 的新增尝试授权优先：可新开原版 Framework 实例，由用户登录实验账号；不终止已有 QQ。
执行边界与结果由 CHECK-005 跟踪；此前静态调查的历史状态保留。
CHECK-005 已完成原版新实例登录及基础状态 API 验证，已有 QQ 进程仍在；没有发送或读取聊天。
这不代表完整同会话交互验收、产品实现或账号安全保证。
CHECK-006 已完成指定会话的 API/窗口发送与对方回复历史对照，以及短时焦点观察；未观察到调用期间抢焦点。
实时事件、更多消息类型、长期并行与恢复仍待验证，不把本次结果升级为通用保证。
不 fork；先澄清许可与原生可审计性，再按 SPEC-003 验证。静态入口存在不等于运行可用。
FACT-003 保留原版入口为有条件原型候选；当前证据不支持直接进入日用会话试跑。

补充内核证据统一见 CHECK-013：群提及、嵌套转发及共享预算、历史小批次回退、附件输入快照/签名所有权恢复/自有缓存清理/配额/附件写意图集成/send/reply全CLI假会话、媒体复制/字节解码/引用复核、隔离启动构造、资源读取/事件/写入与请求输出Schema、事件归一化（含六类通知）/WS传输/订阅编排及重连上限已合成验证；原生启动与完整媒体未验收；事件CLI/标准输出截止已有假会话证据，原生Ctrl+C触发、恢复补读及真实协议尚待验收。

| RFC-0006 | rfcs/0006-single-source-contracts.md | 已采纳方向 | 类型、Clap与Schema权威边界及迁移 |
| TASK-004 | todos/single-source-contracts.md | 已完成 | 单源契约整改与CHECK-050至CHECK-058 |
| RFC-0007 | rfcs/0007-automation-delivery.md | 已采纳方向 | 自动化及内部上下文边界 |
| TASK-005 | todos/automation.md | 本地完成、远端待验 | 上下文迁移、CI/CD及离线交付验收 |

| CHECK-050 | evidence/single-source-foundation-installation.md | 已验收切片 | 共享响应基础、安装管理实际DTO及派生Schema；整体迁移未完成 |

2026-10-21 契约整改阶段一：共享实际响应容器及安装管理四命令已通过CHECK-050。schemars固定1.2.3，Serialize DTO直接生成序列化Schema；312项完整离线回归通过。LegacyWire、分页与错误任意JSON等仍属迁移期，下一步实例状态/生命周期切片；业务68/100不变，不计整体完成。

| CHECK-051 | evidence/single-source-instance-diagnostic.md | 切片通过 | 实例/诊断实际类型及派生契约 |

| CHECK-052 | evidence/single-source-resource.md | 切片通过 | 实际资源DTO及集合分页 |

| CHECK-053 | evidence/single-source-message.md | 切片通过 | 消息、递归段、历史与导出实际契约 |

| CHECK-054 | evidence/single-source-write-error.md | 切片验收 | 写操作/附件/请求与封闭错误细节 |

| CHECK-055 | evidence/single-source-discovery-envelope.md | 切片验收 | 自省/Skill与强作用域响应，删除通用JSON桥 |

| CHECK-056 | evidence/single-source-event.md | 切片通过 | 实际NDJSON枚举与通知/恢复契约 |

| CHECK-057 | evidence/single-source-input.md | 切片通过 | Clap类型、共享规则及输入Schema一致性 |

| CHECK-058 | evidence/single-source-final.md | 已完成整改验收 | 完整回归、旧游标兼容、发布CLI及结构/打包门禁 |

README用户入口约定（2026-10-22）：核心定位为AI Agent通过Shell调度QQ；优先提供可交给Agent执行的配置请求和自然语言任务示例，手动安装/命令作为补充。公开用户文档保留实际操作要求、明确语义和已确认问题，不混入验证推理、验收流水或泛化免责声明；证据边界保留内部上下文。README已按此整理，本地链接、runtime指引入口、源码包和措辞检查通过；业务与稳定性验收状态不变。

公开入口补充（2026-10-22）：展示名称统一QQ CLI，命令/包名保留qq-cli。README面向GitHub访客，通过复制仓库链接委托本地Agent获取、安装和配置，不要求用户先克隆或使用Git；末尾鸣谢NapCat。当前无远端地址，使用明确替换的仓库链接占位，不虚构URL。文档链接、命令名保全及diff检查通过，patch:chore登记；实现与验收状态不变。

远端部署进度（2026-10-22）：用户明确授权gh创建noctisynth/qq-cli公开仓库并推送；仓库已创建，README配置请求及Releases已替换为实际公开URL。本次提交准备推送main；GitHub Actions及真实Release仍以远端运行结果为准，不提前标记TASK-005端到端通过。

安装入口扩展（2026-10-22）：新增独立scripts/install.ps1，支持Windows PowerShell 5.1及PowerShell 7，在Windows x64从官方仓库latest稳定Release解析固定资产，校验SHA256后只安装qq-cli.exe至用户目录，更新用户/当前进程PATH；重复运行升级，下载或校验失败保留旧程序，支持自定义目录与NoPath。无需Git/gh/Rust/管理员/token，不启动QQ、不安装NapCat。脚本可dot-source供合成测试，离线验证加入质量门禁；README给Agent直接运行入口。当前阶段待实现与验收。

安装与注册表发布阶段完成（2026-10-22）：独立install.ps1从latest稳定Release校验SHA256并安装exe及双许可证，默认用户Programs/qq-cli并去重更新用户/当前PATH；NoPath和自定义目录可用。PS7/Windows PowerShell5.1合成安装、升级、重复、校验/架构/锁定失败保全、ZIP路径隔离、临时清理、PATH及无发布包失败通过；完整编排用假下载验证，不修改用户PATH。真实自产12文件ZIP安装后version/skill冒烟通过，Cargo源码包244文件独立编译通过；actionlint与Semifold配置检查通过。用户已设置CARGO_REGISTRY_TOKEN并选择MIT OR Apache-2.0，发布工作流及Cargo元数据已接入，不读取secret值。真实Release尚不存在，在线latest安装与crates.io实际发布待后续版本流程；不自动合并版本PR。

首次发布推进（2026-10-22）：用户授权核对README/Skill、修复CI、通过后合并版本PR并首次发布crates.io/GitHub Release及在线安装验收。已补齐Skill的安装器、PATH恢复、无配置默认值、导出目录、固定NapCat版本、首次登录与恢复流程；删除内部实验叙述和过时媒体验收状态。Skill三项离线检查通过（导出需正常用户ACL权限）；初次线上质量步骤通过，打包/首次发布仍进行中。未授权新增真实QQ操作。

CI排查阶段（2026-10-22）：旧流水线质量/打包通过，但Semifold创建PR收到422 head invalid且远端无release分支；版本推送原因继续核对。最新流水线98个库测试中原生启动恢复用例出现None unwrap，原始dispatch错误被测试吞掉；修正测试直接报告launch/journal错误并添加仅测试编译的阶段诊断。本地40次定向及98项库测试、Clippy、结构通过，线上根因尚待复核，不标记已修复。单独授权的好友状态说明已发送一次accepted；对象/正文仅忽略材料，查询单次超时可通过临时60秒配置完成；最近会话不支持类型另待查明。

首次发布版本与上游排查（2026-10-22）：用户要求首次稳定发布保持0.1.0，不能按现有默认计划升至0.2.0。已取消运行39663937957的后续流程；取消前完整Rust测试已通过，包括原生启动恢复用例，不能据此宣称偶发根因已修复。Semifold0.3.4及上游main固定提交41fb100449a14018e2b6de1290b97a60c4e5d3c2的planner::bump_with_policy仅在进入命名预发布通道时应用channel-bump=preserve，没有保持稳定版的对应入口；不得用alpha等预发布替代0.1.0稳定版。首次发布实现路径待用户选择上游支持或一次性流程，期间不合并、不发布、不推送触发默认升版的提交。上游ci::force_push_release未注册push_update_reference回调，也未验证远端分支存在，存在服务端拒绝未报告的诊断缺口；本仓旧运行基线缺少main后来新增的workflow令牌环境配置，可能遇到GITHUB_TOKEN修改workflow限制，但没有服务端拒绝原文，不能断言该推测已确认。

Alpha转正验证（2026-10-22）：用户授权隔离验证后使用未发布的0.1.0-alpha.0作为首次稳定发布过渡。隔离仓库复制全部115个changeset，Semifold0.3.4 status及实际version应用均得到0.1.0，post-version锁文件同步成功。正式Cargo.toml及Cargo.lock已同步为0.1.0-alpha.0，第三方依赖不变；含新增条目的116个changeset规划目标仍为0.1.0。允许继续推送、CI、稳定版本PR合并及首次发布，替代此前等待方案选择的暂停；不发布任何alpha版本。首次发布与在线安装仍待远端验证。
