# qq-cli 技术方案

文档编号：PLAN-001。状态：完整交付契约已定义，工程基础已实现，业务待实现与验收。更新：2026-10-18。

许可执行策略（2026-10-19，FACT-002）：独立原创CLI与外部API协议实现不等同复制上游代码；自动下载/运行涉及上游复制/使用条款，不能据下载可用推定无限授权。适用范围未知只作提示，不返回LICENSE_UNCONFIRMED、不要求作者证明或确认参数、不参与技术canStart。固定摘要、完整性、精确兼容和归属检查继续生效。捆绑/镜像/修改/商业用途单列发布审计，不因本次实现视为已许可。

## 产品与交付原则

qq-cli 是供 coding agent 使用的 Rust 命令行工具，自动管理 NapCat 实例，提供个人 QQ 任务操作。
用户继续使用官方窗口，Agent 通过后台 API 操作同一账号会话，CLI 退出不关闭 QQ，不占用输入焦点。
命令以任务组织，不要求 Agent 理解 OneBot action、QQ 内部 ID 映射或 WebUI。

交付单位是下述完整能力、用户文档、Agent 指引与验收证据。任务按依赖实现，部分功能和开发快照不能作为完整产品交付。
范围内不能以 NOT_IMPLEMENTED 或未经验证的功能补齐数量。“完善”以明确契约和验收衡量，不代表所有 QQ 功能、
无限历史、永久兼容或零缺陷。Windows 为目标平台，NapCat 基线 v4.18.38，
固定提交 bbd0d2aca3d470c20b33b1d4f95123335633bf26；其他版本或平台需独立支持证据。

CLI 增加 NapCat 运行环境管理，自动准备配置、启动、检查与有界恢复，复用原版 Framework 和官方 QQ 安装。
不修改、fork 或打包 NapCat，不提供任意 raw action、群管理、
批量营销或自动回复。这些是职责边界，不是分期承诺。
当前已实现目标解析/帮助、基础 schema、紧凑错误、配置/私有状态、HTTP/身份门禁和部分集合读取/续传。
完整领域 schema 和其余业务仍在开发并明确失败，离线证据见 CHECK-013，未运行真实 CLI 会话。
以下是设计契约，并非当前可用功能。已有 API 实验不代替 CLI 验收、长期稳定性或账号安全证明。

## Agent 命令模型

使用普通 Rust 子命令语法，无特殊前缀、默认交互问答或窗口操作。
命令注册表统一生成帮助、JSON Schema 和操作指引，Agent 按任务发现能力，不需要一次读完全部文档。

| 命令 | 唯一职责 |
|---|---|
| status | 当前实例、实际账号、登录/在线状态和版本摘要，不启动、不读取聊天 |
| doctor [--version <version>] | 环境、兼容证据、连接和能力诊断；不加载/启动、不试发或下载 |
| schema list / get <command> | 离线命令目录及完整输入/输出/错误 Schema |
| skill list / get <task> / export | 任务指引及通用 Skill 包 |
| napcat install / installations / uninstall | 准备、列出及卸载本地完整安装 |
| napcat start / instances / stop / restart / remove | 复用/明确新开、列实例、停止/重启、注销已停止实例 |
| napcat update / rollback | 明确维护所选实例的安装引用及状态迁移 |
| contact list [--query <text>] / get <user-id> | 好友发现、关系与用户资料；不以是否最近聊天限制发现范围 |
| group list [--query <text>] / get <group-id> / members <group-id> | 所属群发现、群资料与成员；不改变群状态 |
| conversation list [--query <text>] / get <conversation> | 最近聊天及会话信息；过滤始终限定最近会话，不切换为好友/群全量搜索 |
| message get <message-ref> [--expand] | 读取消息及媒体元数据；按需展开合并转发 |
| message history / search | 面向读取的分页历史和范围检索，返回 JSON 与续传游标 |
| message export | 面向文件的历史/检索导出、检查点及续写，stdout 只返回产物与范围摘要 |
| message send / reply <message-ref> | 新消息和引用回复，独立任务入口，共享文本/提及/图片/文件及防重实现 |
| media download <media-ref> / prune | 显式下载、自有缓存清理；不另设 inspect/cache status |
| event watch | 有界事件流，单数资源名与其他命令一致 |
| request get <request-id> | 本地发送/实例创建记录查询，不重发 |

contact 管好友，group 管群与成员，conversation 管聊天会话，message 管消息内容与操作。
contact/group list --query 在各自完整发现范围内有界过滤，返回 ID 和可直接用于消息操作的会话标识；
conversation list --query 仅过滤最近聊天，响应明确 scope=recent，不因参数悄悄扩大到所有好友和群。
contact get/group get 返回对象资料及关系；conversation get 返回会话标识、最近活动等可用会话元数据，
不以会话查询冒充完整用户/群资料，不创建关系或标记已读；上游未提供的信息明确缺失，不推算为事实。
同名仍返回候选，不自动发送；具体对象或消息引用已知时可直接操作，不强制先调用发现/详情命令。

资源目录统一 list/get；installations、instances 保留已确认且意义清晰的安装/实例枚举名称。
不新增兼容别名，也不将诊断、schema 或 dry-run 设为每次任务必经步骤。
解析与帮助已迁移；命令出现不等于业务已实现，本表是持续实施的唯一目标命令面。

常见任务示例（语法待实现，ID 为合成值）：

```text
qq-cli status
qq-cli group list --query "测试群"
qq-cli group get 10001
qq-cli conversation list --query "最近讨论"
qq-cli message history --conversation group:10001 --limit 50
qq-cli message reply <message-ref> --text-file reply.txt --request-id task-001
qq-cli message send --conversation private:10002 --image photo.png --text "图片说明" --request-id task-002
qq-cli message get <message-ref> --expand
qq-cli media download <media-ref> --output attachment.png
qq-cli message search --conversation group:10001 --query "部署" --since 2026-10-01T00:00:00+08:00
qq-cli message history --cursor <opaque-cursor>
qq-cli message export --conversation group:10001 --output history.ndjson
qq-cli message export --output history.ndjson --resume
qq-cli event watch --conversation group:10001 --timeout 30 --limit 100
```

安装和多实例管理按需使用：

```text
qq-cli napcat start
qq-cli napcat installations
qq-cli napcat install --version v4.18.38
qq-cli doctor --version v4.18.38
qq-cli napcat start --new --request-id experiment-001
qq-cli napcat instances
qq-cli --instance <instance-id> napcat stop
qq-cli napcat remove <instance-id>
qq-cli napcat uninstall <installation-id>
qq-cli schema get message.send
qq-cli skill get send
```

send 使用 --conversation 指定新消息目标；reply 使用位置参数 message-ref 推导目标会话与实例，不重复填目标。
send 不再兼作回复入口，不保留 --reply-to 别名；二者共享载荷校验、防重和写入状态机。
文本 --text、--text-file、--stdin 互斥，可与重复的 --image/--file/--mention 组合；至少有文本、媒体或提及内容。
不公开任意消息段 JSON 输入；由 CLI 构造受支持的类型化载荷，未知后端段不转成隐式动作。
文本为有效 UTF-8、保留换行，拒绝空文本及超过 64 KiB，不自动拆分；CQ 外观文本仅为 text。
send/reply 文件发送校验摘要，群提及校验成员，均必填 --request-id；同键改变 send/reply 操作类型视为冲突。
--dry-run 为可选预览，不写入/上传/占用请求键，不要求 request-id，不自动变成真实发送前的第二次调用。
start --new 也必须请求键，以相同键复用同一次实例创建，解决跨进程锁不能防止串行重试多开的缺口。
请求键跨消息写入与实例创建唯一；另一类操作已占用时返回REQUEST_CONFLICT，不以同键创建第二种操作。旧冲突记录保留并拒绝猜测选择。
其余自然幂等的状态/安装/普通 start 不要求额外键；停止、升级等维护命令仍有状态和副作用检查。

`request get KEY`只查询本地持久结果，不发送消息或启动实例。消息结果保留原账号作用域；创建结果返回`operation: napcat.start`、原`instanceId/runId`、持久`outcome`、`instanceState`与下一步，不填账号作用域。`dispatched`只表示调度回调返回，`instanceState`仅为本地登记，`identityVerified`始终为false；实际在线状态须由`status`核对。原实例缺失或换run仍返回原请求身份，并指向`napcat instances`；签名损坏、创建/调度目标不符或显式实例冲突明确失败。查询不补建密钥、不重放pending/unknown。

## 参数和调用成本

所有列表和事件使用 --limit 表示本次返回总数，--timeout 表示命令总截止秒数；
--since/--until 限消息时间，--cursor 续传同一查询。续传从游标恢复目标，不要求重复会话/时间/方向/实例，
显式选项冲突返回错误，不悄悄覆盖游标作用域。
不公开 --max-pages/--max-items/--max-events、后端批次或分页延迟，避免 Agent 调参才能取够一页。
默认列表/历史/搜索 limit=20，文件导出 limit=1000，事件 limit=100；范围 1 至 1000；长查询/流默认 timeout=30，范围 1 至 300。
每次请求仍受独立 10 秒默认/1 至 60 秒配置上限约束，不得超出剩余命令总截止。
查询默认扫描最多 1000 条、100 次后端取页、4 MiB 归一化输出，达到任一预算即明确停止；不是后端容量声明。
预算属于客户端配置和 schema，不用无限 --all 代替。历史仅在传输失败时按20→10→5→2缩小批次，每轮保持原锚点；最小2避免包含锚点的页面无法推进。至多4次尝试，失败尝试也计入page_limit和backendPages，共享总截止；认证/协议/业务拒绝不自动重试，耗尽保留游标而不当作历史终点。

满足本次返回页的 --limit 为正常成功，可以 complete=false 并带 nextCursor，不把普通分页报成错误。
只有在本次页未完成时遇到预算/超时/停滞/传输失败才返回部分结果及退出码 7；
无数据且失败返回对应错误。后端可达终点不是全库完整证明。
search 保留独立命令，因为匹配数量和扫描数量不同于按时间取历史，结果始终说明扫描范围。
接口优先按资源和任务职责拆分，不以命令数量最少为目标；共用机制不等于合并公共语义。
成本通过默认值、引用推导、紧凑输出和按需帮助降低，不靠混合查询范围或为单个命令增加模式参数。

## 自动选择、身份与连接

默认由 qq-cli 管理环境与运行状态，不暴露 profile、managed/external、预绑定或改绑流程。
外部端点接入没有已确认需求，不纳入当前产品必需范围；普通运行中的 QQ 仍不能因此直接接管。

业务命令优先使用显式 --instance，或消息引用/游标内已确定的实例与账号作用域；
否则只有一个候选实例时自动选择，多个候选返回 INSTANCE_AMBIGUOUS 和最少候选信息。
不能用“最近活跃”“登录最久”或“只有其中一个就绪”静默绕过多实例歧义。
没有实例时在已授权启动范围内自动 ensure-ready；无权启动或准备受阻时返回下一步，不询问交互问题。
只读 status/doctor/installations/instances 不启动或改动进程；实际聊天读取前确保版本、身份、在线及策略已通过。

napcat start 默认复用唯一的现有实例，无实例则准备并启动；多实例须指定 --instance。
显式 --new --request-id <key> 表示创建额外实例，同键复用登记的创建结果；创建记录及跨进程锁防并发和串行重试重复。
目标冲突即拒绝，结果不明先 request get 查询，不自动再创建或结束未知归属进程。
--version 选择固定安装版本；已有实例版本不同时明确报冲突，不借 start 隐式更新。
新实例无需预填账号；未登录 account=null、state=login_required，实际身份只来自该实例 API，
不从昵称、模型猜测或启动参数推断。用户可扫码、密码或点击已有账号登录，CLI 不接收密码。
跨调用检测账号变化时中止旧上下文操作并返回 ACCOUNT_CHANGED；旧账号消息引用、游标及请求键不能用于新身份。
Agent 重新 status 获取实际作用域，再依任务授权判断是否继续，不提供公开绑定/改绑配置。
检测仍不能消除检查与执行间的上游切号竞态，也无法识别用户未表达的主观登录意图。

实例 ID 是稳定选择句柄，停止/重启不改句柄；具体进程与运行代次由内部注册表区分。
消息引用与游标带身份/运行有效性校验，不能把稳定实例句柄解释为上游短消息 ID 跨重启稳定。
多实例结果只返回 instanceId、state、实际 account 和必要版本信息，不输出凭据、原生路径或内部注册表。
普通命令默认只返回任务需要的数据与简短作用域，诊断细节由 doctor/schema 按需获取，避免重复帮助和大段自动引导。

配置 schemaVersion=1，仅保留全局预算/策略及内部安装、实例登记，不要求 Agent 创建命名配置。
标准用户数据目录可自动初始化；显式 --config 或 QQ_CLI_CONFIG 可以覆盖，不搜索其他工作区/QQ 私人目录。
自动选择并登记回环端点、随机凭据和用户级受保护存储；相对状态位置以配置文件目录解析。
配置原子写入并检测竞争，损坏则明确报错，不自动重建覆盖；端点/账号不允许环境变量静默替换。
HTTP 只接受显式端口的 http://127.0.0.1 或 http://[::1] 根路径，WS 为对应回环 ws 根路径，
需验证 HTTP/WS 同实例同账号；拒绝用户信息、查询串及路径前缀，禁用代理/跳转/传输层自动重试。
凭据不进入命令参数、URL、stdout 或日志；默认单请求 10 秒，范围 1 至 60 秒，JSON 上限 4 MiB，长操作另设总预算。

授权策略内部限制 read/send/media-download/events 及可选目标/文件范围，沿用用户任务授权，不要求每次配置或弹窗。
无写权限不发信，不因收到消息或重试扩大权限；这不是对同用户恶意进程的隔离边界。
聊天、昵称、文件名和上游错误都是不可信数据，不能变成 shell 命令或 Agent 指令，不自动回复或执行收到的指令。

## NapCat 安装与实例

安装是经过来源/版本/摘要核对的磁盘产物，installationId 标识一份确定内容；实例是使用某份安装的运行资源，
instanceId 独立于 installationId 和 QQ account。一份安装可被多个实例引用，一个实例在某次运行只使用一份安装。
列表不混合：installations 列安装 ID、版本、必要的平台/架构及引用数，instances 列实例 ID、状态、实际账号和安装 ID。

| 命令 | 对象与副作用 |
|---|---|
| napcat install --version <version> | 下载/校验/准备完整安装，已有相同产物则幂等复用，不启动 QQ |
| napcat install --version <version> --from <archive> | 导入本地包，仍核对来源/许可/版本/摘要，不执行下载脚本，不修改原文件 |
| napcat installations | 只列本工具登记的本地安装，不列所有远程 release，不启动进程 |
| napcat uninstall <installation-id> | 删除本工具拥有且无实例引用的安装；被引用返回 INSTALLATION_IN_USE 和实例 ID |
| napcat start [--version <version>] [--new --request-id <key>] [--verify-compatibility] | 复用或明确新开 QQ/NapCat 实例，自动登记；verify-compatibility仅适用于显式新实例的首次组合试验，不登记成功兼容证据；安装缺失时按既定来源流程准备 |
| napcat instances | 列本工具登记的实例，包含停止或等待登录状态，核对进程身份，不扫端口 |
| napcat stop | 显式结束所选签名实例的私有进程组及共享QQ窗口，保留安装、凭据、聊天/发送记录；完成证明可复用 |
| napcat restart | 同一生命周期锁内先核对启动输入、再停止原run并准备后继，保留实例ID及安装版本；未知结果不重放 |
| napcat remove <instance-id> | 凭原签名停止完成证明注销、释放安装引用；退役精确匹配的默认活动指针，保留凭据、运行资料、QQ数据和发送防重记录；重复复用完成记录 |
| napcat update --version <version> / rollback | 明确维护所选实例的版本引用、配置迁移及回滚，不原地覆盖共享安装 |

卸载不自动停进程、不提供绕过在用检查的 force；停止实例仍保留其安装引用以便重启。
`restart`绑定调用选择的原run；原快照的并发调用通过原停止证明和后继来源复用，不重新停止后继。后继准备中断可恢复，已调度未知先检查`status`/`doctor`，不要自动重复`restart`。新调用选到已完成后继并再次重启是新的明确维护操作。重启可能重新要求窗口登录，账号不得沿用旧观察；真实QQ兼容与完整故障矩阵仍需另行验收。
需注销停止实例或明确切换引用后才能卸载；卸载只作用本工具拥有的目录，原始导入文件和官方 QQ 安装不动。
安装/卸载与启动/停止互相独立；所有注册、引用检查和目录操作须跨进程同步，防止卸载与启动竞争。
`napcat start`复用已就绪实例时，重新验证签名运行记录、版本和在线账号，返回`instanceId`、`reused: true`及账号作用域；不会启动额外进程。显式版本不一致返回`INSTANCE_VERSION_CONFLICT`，账号变化返回`ACCOUNT_CHANGED`，由`status`确认实际账号后继续。`--new`始终走新建检查，不隐式复用。当前新建生命周期仍在开发；许可未确认不作运行时硬阻断，见FACT-002。

待登录阶段的`status`可在账号API未就绪时，通过已有签名控制绑定提供`loginObservation`：`phase`固定枚举及`isLogin/isOffline/qrLoginAccepted/coreReady`布尔提示。`account`仍为`null`、`identityVerified`仍为`false`，不返回账号scope、二维码URL或后端错误正文，不修改登记。控制`ready`不等于QQ身份就绪；后续`status`须由OneBot确认实际账号。缺控制绑定的旧记录仍返回未验证状态，不猜端点；已存在但损坏/缺失的原材料、2FA和鉴权拒绝返回错误。仅待登录的连接拒绝（尚未发送请求）或缺身份可观察控制阶段，版本/协议/归属/鉴权错误不降级。网络前后核对完整实例与安装快照，状态锁不跨网络等待。`schema get status`覆盖此分支与已识别账号分支；当前证据为原创服务离线验收，真实原版登录恢复另行验收。

在已有受检安装和精确兼容证据齐备的条件下，普通`start`可从零实例自动创建，无需提供账号或请求键；中断后沿用本地创建检查点，只有匹配原实例和运行身份的准备才能继续。唯一待登录实例返回`AUTH_REQUIRED`、空账号和定向`status --instance`指引，不重复启动；未知结果不自动重放。损坏或不匹配的检查点保留并拒绝，不能通过普通`start`复活已注销实例。缺安装的自动获取、二维码控制及完整恢复仍在开发，当前真实启动门禁仍未满足。
处于`starting`或`login_required`的实例若已有签名服务绑定，`status`可通过受检通道确认实际账号和在线状态，并在登记未并发变化时转为`ready`；离线身份不会提升状态。首次服务发现只使用创建进程前签名的QQ映像、端点和Job信息，对固定端点作一次最多2秒且受总截止限制的零字节归属验证，通过后先保留Job句柄再登记绑定；不发送认证或API，已有绑定直接恢复。缺签名发现输入时只报告未验证阶段，不猜测或探测端点。版本不匹配或实例/安装快照已变化时拒绝更新；真实QQ首次发现兼容、二维码和完整新建仍未完成。
安装变化不改变其他实例，回滚 NapCat 不承诺回滚 QQ 自身数据。

选择完整 Framework 包的配套 EXE/DLL/CJS/MJS/原生依赖，不混装版本；原版 BAT 没有通用版本切换参数。
省略版本时固定本项目审核的兼容基线，不每次解析 latest，不轮试版本寻找可启动组合。
同版本但内容/平台不同不得静默覆盖；登记摘要和 installationId 消除歧义。
解包先固定私有快照并验证整包长度/SHA-256；压缩包64MiB、展开总量512MiB、单成员128MiB、4096成员和32层为内部硬上限。
安装先核对固定版本目录，再访问来源或初始化凭据。输入按需打开，已有受检缓存不重新读取导入文件或下载。下载仅允许HTTPS默认端口，初始来源为固定GitHub发布地址，最多3次显式跳转至GitHub或release-assets.githubusercontent.com；禁止URL凭据/片段、代理和自动重试，使用同一总截止。下载内容仍须通过整包长度和摘要校验，不能以HTTP成功替代产物检查。当前许可解释边界见FACT-002，仅作信息；真实下载验收仍待完成。
拒绝路径穿越、Windows别名/ADS、大小写冲突、链接与特殊文件，不执行包内脚本；失败清理本次工具自有临时目录。
所有权清单签名绑定安装目录、来源、完整文件摘要及目录；登记前中断只复用同一已验证产物，不覆盖不明目录。
卸载先核对实例引用，再持久化移除日志并隔离目录；只删除清单中匹配的成员。文件占用或提交失败可按日志重试，未知新增/修改文件会保留并明确失败。
可写运行目录、配置、日志、凭据和端点按实例隔离，不能共享产物内可写 config；具体机制须验证。
NapCat 目录隔离不能推定 QQ 数据隔离，同账号多实例冲突不得自动注销或挤掉已有会话。

兼容检查键为 release/提交/产物及加载器摘要、QQ build/安装选择、OS/架构和启动模式。
只读 doctor 按证据返回 supported/unsupported/unknown，详情按需包含 source、testedCapabilities、limitations、checkedAt。
预检不注入或试发，未知组合不冒充支持，默认阻止首次自动加载；需另有验证流程及真实实验授权。
新建启动的输入检查检查唯一完整版本安装及摘要、产物完整性和QQ安装候选；最多16个候选，超限、多安装或任一无法核对都拒绝。只有精确Framework启动证据可通过兼容检查，不使用Shell实验或API成功替代。此阶段不创建运行意图或访问账号API；其结果不能代替实际调度前的锁内文件复核。当前公开新建流程仍未完成。
QQ 更新、文件摘要或安装变化使旧结论失效；启动后版本、实际账号和 HTTP/WS 就绪是另一层验证。
doctor将产物完整性、适用依据、安装元数据、兼容证据及所选实例API分别报告。诊断采集成功不等于所有检查通过；canStart只说明受检前置条件。
QQ元数据来自被发现的安装，不能冒充当前进程映像；精确键包含原生/进程架构、系统build、QQ安装选择和受检文件摘要，静态或Shell证据不提升为Framework启动验收。
单次状态成功不证明媒体、事件、长期稳定或账号安全。FACT-003 与 CHECK-005/006/009 仅为有限范围证据。

控制面机制见 RFC-0003，不 fork、不自研加载器。已有普通 QQ 未加载入口则 NOT_ATTACHED，
不自动终止或改造；启动/登录的窗口焦点影响须单独验收。
有效登录态下自动就绪，登录挑战返回 AUTH_REQUIRED 及检查点；CLI 退出不关 QQ，
短暂故障优先有界重连，用户活跃时不自动重启/升级。真实实验需另有明确授权。

## 历史与引用

ID 对外为十进制字符串，数字响应按 i64/u64 无损检查，不经 f64；会话为 private:<id> 或 group:<id>。
message-ref 为绑定 实例/账号/内部身份代次/会话的不透明引用，包含后端可用定位信息，不能只靠短 ID 承诺跨重启稳定。
定位不足返回 STALE_REFERENCE；允许明确从该会话历史重新定位，不因 get_msg 失败断言消息不存在。
时间返回 timeSeconds；since/until 使用带时区 RFC3339，区间为前含后不含。

默认最近一页 20 条，--limit 表示总结果数，范围 1 至 1000。固定 newest_first 顺序，依据会话 real_seq 降序，缺少可核对顺序时停止并明确失败；
保留上游顺序元数据，不保证跨页快照。cursor 绑定账号、实例、会话、方向、过滤及适配版本，校验完整性，
不含正文/凭据；拒绝跨作用域或不兼容游标。扩大查询仍受扫描条数、内部取页次数、输出字节和总截止预算，不提供无限 --all。

pageInfo 返回 nextCursor、scanned、stopReason、complete、coverage。
stopReason 区分 limit、scan_limit、byte_limit、deadline、range_satisfied、backend_end、no_progress；
complete 仅在请求范围有覆盖证据时为 true，后端终点不表示 QQ 完整归档。空页、超时、重复页及预算不能混作“全部历史”。
群历史实验表明大批次超时不能作为终点，需小批次回退、锚点去重及停滞检测；私聊/群聊分页分别验证。
读取允许有次数/总截止限制的重试，写入始终不自动重试。

search 在明确会话、时间和扫描预算内匹配拉取的消息，返回实际扫描范围及截断，不声称服务端全库搜索。
message export --conversation <target> --output <file> 导出 NDJSON，可用 --since/--until 限定时间、--query 过滤内容，
不默认下载附件；目标范围与 history/search 共用查询机制，但独立负责文件、安全落盘、私有清单/检查点及续传。
message export --output <file> --resume 恢复目标、账号和查询，不需复制原参数；冲突即拒绝，可调整本次 limit/timeout。
history/search 不兼作文件写入入口；export 默认拒绝覆盖，只有 --resume 才续写。
文件格式 qq-cli-ndjson-v1：每条消息为 {type:"message",message:…}，末行为 {type:"checkpoint",reference:…}。
正文与签名检查点原子提交；恢复核对正文 SHA-256、行数、路径、账号/实例和运行代次，不接受跨状态目录的引用。
累计文件上限64MiB；同目录 .qq-cli-export.lock 只保存工具标识并协调并发，不删除用户输出；未明临时文件不自动恢复或清理。
校验文件摘要、重复锚点、中断和删除，不能保证动态历史快照或绝对不漏；stdout 只输出路径/计数/覆盖摘要，不复制全部正文。
普通查询不持久化正文，不自动展开转发/下载媒体/标记已读；无法保证上游内部读请求毫无副作用。

## 消息与媒体

统一消息模型为 messageRef、conversation、sender、timeSeconds、segments、可选 replyTo。
类型化表示文本、提及、回复、图片、文件、转发元数据；未知段保留类型及 supported=false，不伪装为完整文本或透传原始对象。
转发展开限制深度、节点和总字节；媒体元数据与内容获取分开。

图片/文件发送仅接受显式本地文件，拒绝任意 URL；校验路径、类型、大小、权限和摘要。
写入前固定为本工具拥有的不可变临时副本，防止校验后输入变化；内容指纹使用类型/文件名/大小/SHA-256，不使用随机暂存路径。附件非空且客户端上限128MiB，格式和上游容量另行核验；未知结果保留副本及请求关联。多步上传/发送各有持久状态，
未知副作用步骤不重复。分别报告文件传输与聊天消息接受状态，上传成功不等于发送/送达。
大小、格式及私聊/群接口差异须由固定后端契约验证后写入 schema，不能猜测承诺。

消息元数据由 message get 返回，不自动下载；media download 仅解析所选消息引用的受支持来源，限制目标连接与字节/时间预算，
防目录穿越、符号链接逃逸和危险文件名，拒绝跟随跳转，临时文件原子完成，默认不覆盖、不执行、不打开。
API token 不附加到媒体 URL。来源与路径安全是验收门槛。
本地缓存文件复制上限128MiB，可信根来自受管实例所有权；不能把后端返回路径直接当读取授权。输出包含字节数和SHA-256，不覆盖现有目标。
缓存有显式配额/保留期，status/doctor 按需报告用量。media prune离线清理本工具签名附件副本，--instance仅过滤所有权；仅匹配已确定accepted/rejected请求且超过24小时才删，pending/unknown、缺失记录或不可信成员保留。删除journal支持中断续理，不调用上游 clean_cache 或删除QQ缓存；附件根已有1GiB/256项客户端硬配额，锁内检查与复制，配额不足明确失败，不自动删除未知材料；其他媒体缓存和完整故障验收仍待落地。
QQ 历史、QQ 本地媒体、临时 URL 与 CLI 缓存生命周期分别说明，不承诺昨天的图片或全部窗口消息都能获取。

## 实时事件

event watch 是有界前台流命令，连接显式正向 WS，无守护进程、入站监听或自动后台任务。
传输仅ws字面回环IP/显式端口的root或/event地址，凭据只放敏感请求头；拒绝URL凭据/查询参数/跳转，直连不继承代理或解析DNS。握手阶段累计接收32KiB；成功后每连接总接收64MiB（含握手）、写64KiB，完整消息/帧4MiB、完整消息或控制帧最多10000个，所有I/O共享总截止。
默认 timeout=30 秒、limit=100，允许有硬上限调整；逐行 NDJSON ready/message/notice/gap/summary，
ready 携带 schemaVersion 和实例/账号作用域；数据事件只带 type、必要内容和去重标识，不逐条重复完整包装。
重连仍须核对原作用域，变化即报告 ACCOUNT_CHANGED 并停止，不把流默默切到新账号。心跳不冒充新消息，stderr 不混入流。
自发消息依赖上游上报配置，须核对并报告；不能以历史查询冒充事件支持。

重连有限退避并受总截止限制，不触发发送。检查点及指定会话补读仅尽力修复，不能证明连续就输出 gap。
结束总结数量、重复、恢复范围及原因；Ctrl+C 清理本命令资源，不关闭 QQ。
连接/重连前后核对原账号，持续每5秒复核；最多2次重连，250/500ms有界退避。输出失败立即停止读取，逐条NDJSON换行并flush；总截止后summary仅有1秒输出宽限。进程输出容量1且逐条确认，阻塞超过截止便退出，不再追加错误行；Agent应忽略退出前未完成的末行。
不保证事件恰好一次、永久积压或全账号不漏，由 Agent 宿主管理命令生命周期。
客户端限制单帧4MiB、总10000帧、数据事件最多1000条及累计输出/总截止；重复和心跳只计数，不当作新消息。撤回、好友新增、群成员及管理员变更以最小字段投影；群事件未解析身份返回null。尚未支持的notice或子类型明确supported=false，不透传原始私密对象。

## JSON、错误与可靠写入

skill list/get 是离线入口，不读取配置、凭据或 QQ 状态。任务 ID 为 discover、read、send、media、events、runtime；
list 只返回任务摘要，get 返回共享约束、单个任务指引和对应命令名，完整参数按需从 schema 获取。
skill export --output DIR 导出独立 qq-cli Skill 包，含 SKILL.md 和按任务拆分的 references；拒绝既有目标目录，
不修改 Codex/OpenCode 配置，不隐式安装到任一宿主，也不授予新的真实会话权限。

除 --help/--version 外普通命令 stdout 为一行紧凑 JSON，event watch 为 NDJSON，不随 TTY 改变。
成功为 schemaVersion、ok=true、data，涉及会话另含 meta.scope（instanceId、实际账号）；分页另含 pageInfo。
省略 error=null、data=null、重复 command、日常 invocationId 和多余成功建议；诊断关联号在错误或 doctor 详情按需返回。
失败为 schemaVersion、ok=false、error，已确定作用域仍附 meta.scope；error 包含稳定 code、简短 message、retrySafe；
写入另含 outcome、requestId，只有确需恢复/选择时才给 nextAction/候选，候选有数量上限及分页。
部分结果保留 data/pageInfo 和 error=PARTIAL_RESULT；不隐藏范围不足，也不逐条重复整份错误包装。
发送结果保留 requestId、outcome、messageRef、deduplicated，不隐藏 unknown、作用域、预算或 gap 来节省 token。
不提供 --fields 裁剪 DSL 或默认 raw 后端透传；稳定且紧凑的领域模型和 limit 已覆盖常见输出需求。
stderr 只放显式诊断和进度，默认无正文、密钥或堆栈。
HTTP 2xx 仍验证 OneBot status/retcode/最终结果，缺字段、类型错误、不安全整数不伪装成功。

合成发送结果示例，不含真实账号或正文：

```json
{"schemaVersion":1,"ok":true,"data":{"requestId":"task-001","outcome":"accepted","cached":false,"receipt":{"messageId":"40001","conversation":"group:20001"}},"meta":{"scope":{"instanceId":"example-instance","account":"10001"}}}
```

```json
{"schemaVersion":1,"ok":false,"error":{"code":"WRITE_OUTCOME_UNKNOWN","message":"发送结果未知","outcome":"unknown","retrySafe":false,"requestId":"task-001","nextAction":{"command":"request.get","args":{"request_id":"task-001"}}},"meta":{"scope":{"instanceId":"example-instance","account":"10001"}}}
```

| 退出码 | 含义 |
|---|---|
| 0 | 执行成功；发送仅表示上游接受，不代表送达或已读 |
| 2 | 参数、配置、输入、歧义或请求键冲突 |
| 3 | 连接、鉴权、协议或版本问题 |
| 4 | 账号、在线、策略、能力门禁或上游明确拒绝 |
| 5 | 写入结果未知，禁止自动重发 |
| 6 | 本地存储或内部错误 |
| 7 | 部分查询结果，检查范围、预算与续传 |
| 130 | 用户中断；写入仍须按记录判断 unknown |

request-id 为 1 至 128 个 ASCII 字母数字、连字符或下划线。
请求键在本工具用户状态域内唯一，记录发送/创建的操作类型，类型或载荷冲突即拒绝，request get 无需再填操作类别。
发送防重按本工具用户状态域和真实账号管理，不因端口、安装版本、实例重启或选择别名改变而重置。
已有请求键不能静默跨账号复用；不同实例同账号共享防重域，独立状态目录/设备不具备全局防重保证。
摘要包含契约版本、目标、回复、消息段和附件摘要，不按正文猜测操作身份。
预检 → 独占占用键并可靠落盘 → 单次写调用 → 保存结果 → 输出。
同键同载荷 accepted 返回缓存，不同载荷冲突；pending/unknown，包括崩溃留下 pending，一律不能同键再写。
记录损坏、并发无法安全处理、持久化失败时拒绝发送；仅存摘要、作用域、状态及引用，不存正文/凭据，不自动删除去重记录。

写边界后超时、中断、损坏响应、不确定副作用或保存成功结果失败均 unknown；
明确未尝试为 not_attempted，可靠明确拒绝为 rejected，有有效最终消息 ID 才 accepted。
request get 不提供 force-resend，未知结果需用户核对再作明确决定。
跨设备、删除记录、换新键可能重复，本地去重不是上游 exactly-once。

## 架构与证据

单 Cargo binary crate，edition 2024，Cargo.toml 唯一版本源，Cargo.lock 锁定，Semifold rust resolver。
已验收 Rust 1.98.1 与 clap、serde/serde_json、reqwest blocking、sha2；新增 chrono、HMAC/base64、系统随机数、tempfile 和 Windows keyring，版本由 Cargo.lock 固定。
按 command/schema、config/policy、client/adapter、domain、history、media、events、state、output 分责；
普通同步命令和专用事件循环，WS/文件锁/信号/持久化依赖需先验证 Windows、许可及锁文件。
现有 HTTP 依赖不等于已有流或媒体实现，不为普通调用强制常驻运行时。

| 能力 | 固定后端候选 | 证据状态 |
|---|---|---|
| 状态/身份/版本 | get_status、get_login_info、get_version_info | 原版实验通过；CLI 未实现 |
| 消息/私聊文本 | get_msg、get_friend_msg_history、send_private_msg | 部分窗口对照；引用及私聊分页待验证 |
| 群历史 | get_group_msg_history | 有界分页通过；非完整归档证明 |
| 联系人/群/最近会话 | get_friend_list、get_group_list、get_group_member_list、get_recent_contact | 固定源码存在对应类；字段/运行契约待核对 |
| 群消息/回复/提及/图片 | send_group_msg、send_msg 和消息段 | 候选映射，限制/结果待核对 |
| 文件/媒体/转发 | upload_private_file、upload_group_file、get_file、get_image、get_forward_msg | 候选映射，副作用/路径/大小待核对 |
| 事件 | OneBot 正向 WebSocket | 身份关联、自发上报、恢复待验证 |

capabilities 分开 implemented、backendSupported、runtimeVerified；源码存在不能表示运行通过。
探测仅允许无写入有界检查，不能试发或下载。后端无法满足范围时登记阻塞并解决，不能静默删项称交付完成。

## 完整交付门槛

- 注册表、schema、帮助、示例、指引和实现一致，范围内无占位命令，未知能力和部分结果可机器判定。
- 假服务/子进程覆盖身份竞态、断流、鉴权、预算、整数、时间、游标、重启、并发及隐私。
- 故障注入验证发送/附件多步状态机，崩溃、应答丢失、持久化失败和跨进程同键均无隐式重发。
- 逐项真实回归私聊/群、回复/提及、图片/文件、转发、事件和恢复；账号/目标/内容需另行具体授权。
- 核对官方窗口/API 一致及正常输入焦点，记录版本、时长和限制，不外推长期保证。
- 通用 Agent 指引及安装说明，在 Codex/OpenCode 各验收能力发现、历史读取、歧义处理、发送和未知结果处置。
- 干净 Windows 环境安装/升级保留配置状态；fmt/clippy/test/release 及源码包白名单通过。
- 原生二进制、校验值、完整文档、配置/schema、Agent 包、本项目许可及依赖通知齐全，无上游/研究/私有材料。
- 未解决的许可、能力或验证缺口阻止相应验收；本轮不公开发布或新增真实操作。

## 参考

参考 [Lark CLI 官方仓库](https://github.com/larksuite/cli) 的能力发现、schema 和任务指引，
及 [OpenCode Agent Skills](https://opencode.ai/docs/skills/) 的按需加载；不复制代码或特殊语法。
查阅 2026-10-17，均为可变来源，不代替后端基线或宿主兼容证据。
NapCat 依据固定提交 packages/napcat-onebot/action；实验见 CHECK-006、CHECK-009，许可见 FACT-002。

事件输出契约按需由schema get event.watch读取：outputFormat=ndjson，responseSchema验证每条流记录，preStreamErrorSchema验证ready之前的单行错误，streamProtocol说明ready/summary及输出失败边界。JSON Schema不代替连接身份和时序验证。

contact.list/get、group.list/get/members、conversation.list/get的responseSchema描述领域数据、作用域、分页与错误信封；complete=false的集合页必须携带有效游标，complete=true游标为null。读取群详情同时核对返回群ID，不以返回成功替代目标一致性。

附件保留副本带签名请求和账号作用域所有权；恢复同时核对目录清单及字节摘要。错误请求/账号或未登记文件不接管；持久副本登记与消息写意图是两个步骤，未确认副作用不得靠重建副本重发。

附件合计最多16项，文本/提及后按images、files顺序添加，reply前缀另加。副本在pending后绑定payloadHash，单次send_*发送，不重复调用upload_*。内部文件传输不可单独观察时fileTransfer=not_observable，响应丢失时unknown，都不授权重传。同键重试需可读输入以比较内容；已有结果可直接request get，避免重新准备。

已持久写请求的错误meta.scope保持原请求绑定；unknown.nextAction提供request.get及符合输入schema的request_id，不自动改用当前账号或重发。

reply根据签名消息引用直接推导实例与会话，多实例不要求重复选择；显式--instance与引用冲突时立即拒绝，不读取无关实例凭据。缓存结果来自原请求作用域，不要求后端在线。

业务、status API探测、event和doctor API层共用可信签名服务记录，核对运行轮次、安装摘要、端点与完整进程/Job后才创建受检客户端；记录缺失/损坏或凭据键不同均不回退到普通HTTP/WS。完全匹配的已有回执可离线返回，不因记录或当前服务不可用而重发。合成CLI验证涵盖status/联系人、发送/回复、文件与事件，不替代原版启动和真实协议验收。

schema get message.send/message.reply/request.get提供写入和查询领域输出，区分dry-run、accepted、unknown及附件证据。缓存回执损坏或目标不一致返回REQUEST_RECEIPT_INVALID，不重发；后端返回零/溢出消息ID不能作为接受证据。JSON Schema验证结构，语义校验还核对ID整数范围、目标和附件来源。

媒体路径边界：NAPCAT_WORKDIR/cache与QQ内核下载完成路径不是同一来源。不能仅因API返回本地路径就读取它。原版可通过enableLocalFile2Url返回base64（默认关闭）；候选字节传输保留引用/身份校验和128MiB解码预算，专用字节传输已定向验证，自动配置与真实协议仍待完成，不全局扩大普通JSON响应上限。

当前条件实现：media download已接入显式file_id文件的引用复核、独立编码/元数据响应预算与身份守卫后原子落盘，普通API仍4MiB；无base64返回MEDIA_BYTES_UNAVAILABLE。图片从新鲜原消息同位置元素解析受限HTTPS来源，独立客户端验证证书/主机名，不携带API认证、不走代理/跳转/重试；不使用文件名缓存或任意API路径。仅文件名且无法恢复受检来源时返回MEDIA_BINDING_UNVERIFIED。文件CLI及图片TLS/编排已有合成证据，托管开关、图片完整CLI和真实协议仍待验收。

操作截止从命令入口起算，准备与后端阶段共用同一截止，不因进入查询或发送阶段重新获得完整timeout。检查边界过期即停止下一阶段；同步系统调用仍可能无法立即中断，不承诺所有OS操作可被强制抢占。

启动前材料以签名日志认领私有暂存/发布目录，受保护凭据保存整组秘密，重试复用已保存值。只修复已认领暂存文件的准确中断前缀，不覆盖未知或修改后的字节；完整配置发布后才登记prepared。prepared只表示准备完成，status报告account=null及identityVerified=false，不探测计划端口。业务身份/事件入口拒绝未就绪阶段，已有发送/回复回执可在停止状态下离线查证。实际启动、进程与监听归属、运行后配置恢复仍须单独验收。
