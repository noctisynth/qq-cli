# qq-cli 技术方案

文档编号：PLAN-001。状态：完整交付契约已定义，工程基础已实现，业务待实现与验收。更新：2026-10-17。

## 产品与交付原则

qq-cli 是供 coding agent 使用的 Rust 命令行工具，连接用户已运行的 NapCat，提供个人 QQ 任务操作。
用户继续使用官方窗口，Agent 通过后台 API 操作同一账号会话，CLI 退出不关闭 QQ，不占用输入焦点。
命令以任务组织，不要求 Agent 理解 OneBot action、QQ 内部 ID 映射或 WebUI。

交付单位是下述完整能力、用户文档、Agent 指引与验收证据。任务按依赖实现，部分功能和开发快照不能作为完整产品交付。
范围内不能以 NOT_IMPLEMENTED 或未经验证的功能补齐数量。“完善”以明确契约和验收衡量，不代表所有 QQ 功能、
无限历史、永久兼容或零缺陷。Windows 为目标平台，NapCat 基线 v4.18.38，
固定提交 bbd0d2aca3d470c20b33b1d4f95123335633bf26；其他版本或平台需独立支持证据。

CLI 不管理 QQ 启停、登录、注入、更新，不修改、fork 或打包 NapCat，不提供任意 raw action、群管理、
批量营销或自动回复。这些是职责边界，不是分期承诺。
当前只有工程和离线命令骨架可用，业务命令返回 NOT_IMPLEMENTED，未连接后端。
以下是设计契约，并非当前可用功能。已有 API 实验不代替 CLI 验收、长期稳定性或账号安全证明。

## Agent 命令模型

使用普通 Rust 子命令语法，无特殊前缀、默认交互问答或窗口操作。
命令注册表统一生成帮助、JSON Schema 和操作指引，Agent 按任务发现能力，不需要一次读完全部文档。

| 命令族 | 完整职责 |
|---|---|
| schema list / schema show <command> | 离线查询参数、输入/输出 schema、错误、权限、副作用、示例、预算 |
| capabilities | 本地契约与后端支持证据；运行检查需显式 profile |
| profile list / add / show / remove | 管理端点、期望账号、凭据引用；脱敏输出，删除不清除发送记录 |
| doctor / status / self | 预检、状态、自身账号；不试发消息、不下载媒体 |
| contact list / search / get | 联系人发现、明确 ID 查询、有界名称匹配 |
| group list / get / members | 群及成员发现，不含管理 |
| conversation list / resolve | 最近会话及名称候选；同名返回歧义，禁止自动选择发送对象 |
| message get / history / search / export | 私聊/群聊读取、分页、时间过滤、范围内搜索、可续传导出 |
| message send / reply | 文本、提及、图片、文件；回复绑定同一会话的消息引用 |
| message forward get | 显式展开合并转发，限制深度/节点/字节，不发送合并转发 |
| media inspect / download / cache status / cache prune | 元数据、显式下载、仅本工具缓存的查看清理 |
| events watch | 正向 WebSocket 有界订阅、流输出、恢复和缺口报告 |
| request show | 查询本地发送记录，不重发 |
| agent guide / agent export | 按任务输出指引、导出通用 SKILL.md 包，不自动改写宿主配置 |

示例语法待实现；ID 为合成值：

```text
qq-cli schema show message.history
qq-cli --profile lab doctor
qq-cli --profile lab conversation resolve --query "测试群"
qq-cli --profile lab message history --conversation group:10001 --since 2026-10-01T00:00:00+08:00 --limit 50 --max-pages 10
qq-cli --profile lab message history --cursor <opaque-cursor>
qq-cli --profile lab message send --conversation private:10002 --stdin --request-id task-001
qq-cli --profile lab message reply --message <message-ref> --text-file reply.txt --request-id task-002
qq-cli --profile lab events watch --conversation group:10001 --timeout 30 --max-events 100
```

send/reply 文本来源 --text、--text-file、--stdin 互斥；也可使用类型化消息段 --input-file JSON，与文本来源互斥。
文本有效 UTF-8、保留换行，拒绝空内容及超过 64 KiB，不自动拆分。普通文本只构造 text 段，CQ 外观文本不执行动作。
提及、回复和附件显式表达，群提及校验成员；写操作必须 --request-id。
--dry-run 校验目标、输入、策略与载荷，输出脱敏预览，不调用写接口，不预上传。

## 多账号与连接

配置 schemaVersion=1，包含 profiles；每项 endpoint、可选 wsEndpoint、expectedAccount、tokenEnv、timeoutMs、stateDir、policy。
入口为显式 --config 或 QQ_CLI_CONFIG，不搜其他项目、QQ 安装或实验目录；相对 stateDir 以配置文件目录解析。
profile add 是本地操作，不启动实例或猜账号；凭据只经 tokenEnv 指定环境变量，不进参数、URL、配置正文或日志。
配置写入原子替换、检测并发冲突并限制为用户权限；配置损坏拒绝自动重建。

业务调用使用 --profile；仅用户显式配置 defaultProfile 时可省略，即使只有一个 profile 也不自动选择。
无全局“切换当前账号”命令，多个 Agent 互不改变选择；每项绑定期望账号。
聊天读取/写入前校验版本、身份、在线与策略，status/self 可明确展示不匹配信息。
无法锁定上游运行中切号；操作后复核可发现部分竞态，不能撤销已经发生的操作。

端点由用户配置，不扫描端口或导入 NapCat 密钥。HTTP 仅允许显式端口的 http://127.0.0.1 或 http://[::1] 根路径；
WS 为相应回环 ws 根路径，需验证 HTTP/WS 同账号。拒绝用户信息、查询串及路径前缀，禁用代理、跳转、传输层自动重试。
默认每请求 10 秒，配置范围 1 至 60 秒，JSON 响应上限 4 MiB，长操作另有总预算。
覆盖顺序为命令行非敏感选项、对应非敏感环境变量、profile；端点/账号不允许环境变量静默覆盖。

profile policy 限制 read、send、media-download、events，可附目标白名单、文件根目录及大小上限。
新 profile 默认只读，发送通过明确本地配置启用；不为每次调用弹窗。拒绝结果说明所缺权限及 nextAction。
这不是同用户恶意进程的安全隔离。聊天、昵称、文件名和上游报错是不可信数据，不能成为 shell 命令或 Agent 指令。
不自动回复、转发或执行收到的指令。

## 历史与引用

ID 对外为十进制字符串，数字响应按 i64/u64 无损检查，不经 f64；会话为 private:<id> 或 group:<id>。
message-ref 为绑定 profile/账号/会话的不透明引用，包含后端可用定位信息，不能只靠短 ID 承诺跨重启稳定。
定位不足返回 STALE_REFERENCE；允许明确从该会话历史重新定位，不因 get_msg 失败断言消息不存在。
时间返回 timeSeconds；since/until 使用带时区 RFC3339，区间为前含后不含。

默认最近一页 20 条，limit 范围 1 至 100，max-pages 默认 1。明确 oldest-first/newest-first 顺序，同秒使用稳定定位信息排序，
保留上游顺序元数据，不保证跨页快照。cursor 绑定账号、profile、会话、方向、过滤及适配版本，校验完整性，
不含正文/凭据；拒绝跨作用域或不兼容游标。扩大查询仍受 max-items、max-bytes、deadline 硬预算，不提供无限 --all。

pageInfo 返回 nextCursor、returned、scanned、stopReason、complete、coverage。
stopReason 区分 page_limit、item_limit、byte_limit、deadline、range_satisfied、backend_end、no_progress；
complete 仅在请求范围有覆盖证据时为 true，后端终点不表示 QQ 完整归档。空页、超时、重复页及预算不能混作“全部历史”。
群历史实验表明大批次超时不能作为终点，需小批次回退、锚点去重及停滞检测；私聊/群聊分页分别验证。
读取允许有次数/总截止限制的重试，写入始终不自动重试。

search 在明确会话、时间和扫描预算内匹配拉取的消息，返回实际扫描范围及截断，不声称服务端全库搜索。
export 输出 NDJSON 和清单/检查点，显式指定路径并拒绝默认覆盖；续传校验账号、查询及文件摘要，
处理中断、重复锚点及上游删除，不保证动态历史快照或绝对不漏。
普通查询不持久化正文，不自动展开转发/下载媒体/标记已读；无法保证上游内部读请求毫无副作用。

## 消息与媒体

统一消息模型为 messageRef、conversation、sender、timeSeconds、segments、可选 replyTo。
类型化表示文本、提及、回复、图片、文件、转发元数据；未知段保留类型及 supported=false，不伪装为完整文本或透传原始对象。
转发展开限制深度、节点和总字节；媒体元数据与内容获取分开。

图片/文件发送仅接受显式本地文件，拒绝任意 URL；校验路径、类型、大小、权限和摘要。
写入前固定为本工具拥有的不可变临时副本，防止校验后输入变化；多步上传/发送各有持久状态，
未知副作用步骤不重复。分别报告文件传输与聊天消息接受状态，上传成功不等于发送/送达。
大小、格式及私聊/群接口差异须由固定后端契约验证后写入 schema，不能猜测承诺。

inspect 不自动下载；download 仅解析所选消息引用的受支持来源，限制目标连接与字节/时间预算，
防目录穿越、符号链接逃逸和危险文件名，拒绝跟随跳转，临时文件原子完成，默认不覆盖、不执行、不打开。
API token 不附加到媒体 URL。来源与路径安全是验收门槛。
缓存有显式配额/保留期，只清理本工具登记拥有且不在用的文件，不调用上游 clean_cache 或删除 QQ 缓存。
QQ 历史、QQ 本地媒体、临时 URL 与 CLI 缓存生命周期分别说明，不承诺昨天的图片或全部窗口消息都能获取。

## 实时事件

events watch 是有界前台流命令，连接显式正向 WS，无守护进程、入站监听或自动后台任务。
默认 timeout=30 秒、max-events=100，允许有硬上限调整；逐行 NDJSON ready/message/notice/gap/summary，
带 schemaVersion、profile、账号、invocationId 及去重标识。心跳不冒充新消息，stderr 不混入流。
自发消息依赖上游上报配置，须核对并报告；不能以历史查询冒充事件支持。

重连有限退避并受总截止限制，不触发发送。检查点及指定会话补读仅尽力修复，不能证明连续就输出 gap。
结束总结数量、重复、恢复范围及原因；Ctrl+C 清理本命令资源，不关闭 QQ。
不保证事件恰好一次、永久积压或全账号不漏，由 Agent 宿主管理命令生命周期。

## JSON、错误与可靠写入

除 --help/--version 外普通命令 stdout 为一行 JSON，流/导出为 NDJSON，不随 TTY 改变。
--fields 仅裁剪 data，保留错误、作用域及分页；stderr 仅显式诊断/进度，默认不输出正文、凭据或堆栈。
包装 schemaVersion、ok、command、data、error、meta；meta 含 invocationId、profile、账号，写入另含 requestId。
error 为稳定 code、message、outcome、retrySafe、nextAction。成功 error=null；失败通常 data=null，
部分查询使用 ok=false、PARTIAL_RESULT、已取得 data 及续传信息，退出码 7。
HTTP 2xx 必须继续核对 OneBot status/retcode/结果结构；缺字段、类型错误、不安全整数和异步非最终应答不能冒充成功。

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
命名空间绑定规范端点及期望账号，同实例同账号 profile 别名须共享去重域、检测 stateDir 冲突。
摘要包含契约版本、目标、回复、消息段和附件摘要，不按正文猜测操作身份。
预检 → 独占占用键并可靠落盘 → 单次写调用 → 保存结果 → 输出。
同键同载荷 accepted 返回缓存，不同载荷冲突；pending/unknown，包括崩溃留下 pending，一律不能同键再写。
记录损坏、并发无法安全处理、持久化失败时拒绝发送；仅存摘要、作用域、状态及引用，不存正文/凭据，不自动删除去重记录。

写边界后超时、中断、损坏响应、不确定副作用或保存成功结果失败均 unknown；
明确未尝试为 not_attempted，可靠明确拒绝为 rejected，有有效最终消息 ID 才 accepted。
request show 不提供 force-resend，未知结果需用户核对再作明确决定。
跨设备、删除记录、换新键可能重复，本地去重不是上游 exactly-once。

## 架构与证据

单 Cargo binary crate，edition 2024，Cargo.toml 唯一版本源，Cargo.lock 锁定，Semifold rust resolver。
已验收 Rust 1.98.1 与 clap、serde/serde_json、reqwest blocking、sha2。
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
