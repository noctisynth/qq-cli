# qq-cli 首版技术方案

文档编号：PLAN-001。状态：M0 implemented，M1 至 M4 planned。更新：2026-10-17。

## 目标和交付边界

qq-cli 是面向本地 Agent 的 NapCat OneBot HTTP 客户端：解析命令、校验目标账号、请求固定接口，
输出稳定 JSON。QQ 窗口与会话由用户已经启动的 NapCat 实例维护。
首版不修改或打包 NapCat，不处理 DLL、QQ 启停、登录、更新、注入或窗口自动化。
CLI 退出仅结束当前命令，不关闭 QQ。产品支持先限 Windows；其他平台待验证。

现有 v4.18.38 实验已通过私聊文本收发记录对照及短时焦点观察；这不是 CLI 验收，
也不是所有 QQ 版本、长期稳定性或账号安全保证。版本不匹配必须明确报告，不能暗示已受支持。

## 实现路线

按用户决定采用 Rust，单个 Cargo binary crate、edition 2024，交付 qq-cli 原生可执行文件。
Cargo.toml 承载唯一包版本，已提交 Cargo.lock；工具链与 rust-version 当前固定为经验证的 Rust 1.98.1。
参数解析采用 clap derive，JSON 采用 serde/serde_json，HTTP 采用 reqwest blocking，摘要使用 sha2。
首版同步顺序执行，不自行建立异步任务调度或常驻运行时；reqwest 内部运行机制不等于 CLI 提供异步服务。
reqwest 关闭默认 features，仅开启 blocking/json；当前只连接回环 HTTP，不引入 TLS、压缩或通用 Bot SDK。
依赖精确解析结果由 Cargo.lock 固定；M0 已核验 clap 4.6.7、serde 1.0.229、serde_json 1.0.154、reqwest 0.13.5、sha2 0.10.9。
测试使用 Rust 内置测试和标准库子进程/本地监听器；这些依赖的锁定与编译不等于后端功能已实现。
CLI 与 QQ 自带 Electron/Node 分属不同进程，不需要匹配 QQ 的原生模块 ABI，用户无需安装 Node.js。
M0 已建立 Cargo 工程并将 Semifold 解析器迁移为 rust，移除 package.json；没有双版本来源。

| 模块 | 职责 |
|---|---|
| CLI 入口 | 子命令、参数、stdin、退出码和帮助 |
| 配置 | 显式配置文件、环境变量、校验和凭据脱敏 |
| OneBot 客户端 | 鉴权、超时、响应体限制、协议检查 |
| 用例 | 状态、自身账号、消息读取与文本发送；固定 action 白名单 |
| 发送记录 | 跨 CLI 进程的请求键占用、载荷摘要和已知结果 |
| 输出 | 版本化 JSON 包装、统一错误和消息字段归一化 |

首版无常驻服务、数据库、实时事件连接或后台轮询。后续事件订阅独立设计 NDJSON、重连及缺口语义。

## 最小命令和上游映射

以下命令的解析与帮助已在 M0 实现，业务语义仍待 M1 至 M3；统一可执行名为 qq-cli。
当前有效业务命令返回 NOT_IMPLEMENTED、outcome=not_attempted、退出码 6；参数错误返回 INVALID_ARGUMENTS、退出码 2。
M0 不加载配置、不打开消息文件、不读取 stdin、不发起网络请求；消息文本及未知参数不会被默认诊断回显。

| 命令 | OneBot action | 语义 |
|---|---|---|
| doctor | get_version_info、get_status、get_login_info | 检查配置、鉴权、版本、在线和账号匹配；从不试发消息 |
| status | get_status | 返回在线状态；离线作为状态数据，不伪装为网络错误 |
| self | get_login_info | 显式返回当前账号标识与昵称，不自动写入期望账号 |
| message get --id <message-id> | get_msg | 读取已知消息；ID 不保证可跨上游重启使用 |
| message history --user <user-id> [--limit 20] | get_friend_msg_history | 只取指定私聊最近 N 条，N 范围 1 至 100，不自动翻页或扫全部会话 |
| message send --user <user-id> --text <text> --request-id <key> | send_private_msg | 单次纯文本发送；请求键必填，发送前核对账号与在线状态 |

send 文本可用 --text-file <file> 或 --stdin 替代 --text；三种来源严格互斥。
文件和 stdin 必须是有效 UTF-8，保留换行和原始文本，不经过 shell 拼接；拒绝空文本与超过 64 KiB 的 UTF-8 输入。
这个限制只是 CLI 资源上限，不表示 QQ 接受所有该长度文本；首版不自动拆分长消息。
以 text 消息段数组传给上游，避免 CQ 字符串把普通文本解释为其他消息类型。
历史查询固定 disable_get_url=true、parse_mult_msg=false、quick_reply=false，不自动展开转发、下载媒体或标记已读。
上游仍可能有内部读请求副作用，不能承诺被动旁路。

群聊、联系人枚举、图片/文件、撤回、转发、搜索、raw action、WebUI 管理、实时推送均不进入最小实现。
不做账号/昵称模糊匹配；消息命令必须给出目标数字标识，禁止默认为最近会话。

## 配置与连接

配置为 JSON，包含 schemaVersion=1、endpoint、tokenEnv、expectedAccount、timeoutMs、stateDir。
首版只读显式 --config <file> 或 QQ_CLI_CONFIG 指定的文件，不搜索其他项目或实验目录。
连接地址可由 QQ_CLI_ENDPOINT 覆盖；密钥从 tokenEnv 指定的环境变量读取，默认变量名 QQ_CLI_TOKEN。
期望账号从配置 expectedAccount 读取，不用环境变量静默覆盖；任何账号修订都需要显式修改配置。
stateDir 为本地发送记录目录，必填；相对目录以配置文件所在目录解析，不以当前 shell 目录解析。
不复制、猜测或自动导入 NapCat 的配置与鉴权信息。token 不放在命令行参数、URL、仓库示例或输出中。

只接受带显式端口的 http 回环字面地址 127.0.0.1 或 [::1]，不接受域名、远端、用户信息、查询串或路径前缀。
只允许根路径，禁用重定向；连接/读取均有截止时间，避免凭据跟随跳转或隐式代理发送到其他地址。
reqwest 显式设置 no_proxy、禁止重定向与禁止自动重试，并配置连接/整体超时；不能依赖库的默认行为。
默认每请求 10 秒超时，可配置 1 至 60 秒；响应体上限 4 MiB，超限返回明确错误。

doctor、status、self 不要求预先知道当前账号；消息命令必须配置 expectedAccount。
每个消息命令在读取聊天或发送之前调用 get_login_info 和 get_status，确认账号一致且 online=true。
doctor 对离线、账号不匹配或非 v4.18.38 返回不就绪；status 的成功输出仍可携带 online=false。
首版仅支持已验证版本，其他版本返回不兼容，不自动修改后端。doctor 的检查结果不是完整 capability 探测。
固定版本 get_status 的 good 字段由实现直接返回 true，不能单独视为所有接口、网络或窗口健康证明。
所有业务命令在执行前核对上游版本；无法在检查与操作间锁定 QQ 会话，发生账号切换的竞态仍是已知限制。

## JSON 和错误契约

除 --help 和 --version 外，所有命令默认向 stdout 输出一行 JSON，包括失败；不因 TTY 改变输出格式。
进度或显式调试信息仅进 stderr，默认不输出请求/响应正文、Authorization、token、聊天文本或原始堆栈。
输出使用 schemaVersion=1、ok、command、data、error、meta；成功时 error=null，失败时 data=null。
meta 至少包含本次调用的 invocationId，发送时另有调用方 requestId；二者不能混用。
error 使用稳定 code、简短 message、outcome；已校验的上游 retcode 可作为数值详情，原始 wording 不直接透传。
未知字段忽略；缺少本命令必需字段、错误类型、非 JSON 或不安全整数返回协议错误，不能填默认值冒充成功。
HTTP 2xx 不等于成功：必须同时确认 status=ok、retcode=0 及命令响应结构；异步应答不当作最终发送成功。

账号、消息、会话标识均对外输出十进制字符串；请求尽可能传字符串，数字响应仅在能无损解析为整数时转换。
用户标识为正十进制，消息标识允许有符号十进制；serde_json 数字按 i64/u64 检查，不经 f64 转换，超范围或小数拒绝静默取整。
时间保持上游 Unix 秒的明确字段单位，不混入毫秒。history 返回上游顺序和 order=upstream，不假定按时间排序。
消息结果保留 id、senderId、timeSeconds、文本段；不支持的段仅保留 type 与 supported=false，不触发附件解析。
没有 text 段时不能伪造空白文本消息；以 segments 表示实际类型。

| 退出码 | 错误类别 |
|---|---|
| 0 | 命令执行成功；发送仅表示上游已接受，不代表对方收到或已读 |
| 2 | 参数、配置、输入或请求键载荷冲突 |
| 3 | 连接、超时、HTTP 鉴权或协议/版本错误 |
| 4 | 离线、账号不匹配、上游业务拒绝或 doctor 未就绪 |
| 5 | 发送结果未知，禁止自动重发 |
| 6 | 本地状态存储失败或非预期内部错误 |

发送错误的分类优先按下面的发送状态规则，不能因为底层是超时就一律返回退出码 3。

例如结果未知时输出以下合成示例，不包含正文或鉴权信息：

```json
{"schemaVersion":1,"ok":false,"command":"message.send","data":null,"error":{"code":"SEND_UNKNOWN","message":"发送结果未知，请核对会话，勿自动重试","outcome":"unknown"},"meta":{"invocationId":"example-invocation","requestId":"example-request"}}
```

## 发送与重试

首版所有请求均不自动重试；读命令失败后调用方可重新执行。
send 的 request-id 由调用方为同一逻辑发送复用；不根据文本内容推断重复，也不假装上游支持幂等键。
发送记录命名空间为规范 endpoint 与 expectedAccount；内容摘要包含目标、原始文本字节和命令契约版本。
请求键先校验长度 1 至 128、限定字母数字及连字符/下划线；用于磁盘文件名时仍做摘要，不拼接未校验路径。

流程：参数/版本/会话校验 → 独占创建请求记录并落盘 → 单次网络请求 → 落盘已知结果 → 输出。
记录只保存摘要、账号/目标标识、时间、状态及消息 ID，不保存正文或 token；属于用户私有状态，不提交 Git。
同键同载荷已有 accepted 结果时返回已存结果并标记 deduplicated=true，不再次发信；不同载荷返回冲突。
同键仍为 pending 或结果 unknown 时返回 SEND_UNKNOWN，不判断为可重试；进程崩溃留下的 pending 也如此处理。
记录损坏、写入失败或跨进程竞争无法安全处理时拒绝发送；不能删除旧记录以悄悄重发。

进入网络发送边界后，超时、中断、响应损坏、上游错误且无法确认是否发送，均保守记录 unknown。
可确认的 HTTP 鉴权拒绝可记录 rejected，但同一进程仍不重试；发送前参数/会话错误为 not_attempted。
只有有效最终成功应答且有消息 ID 才记 accepted；保存 accepted 失败时按未知结果处理，不能报告可安全重试。
客户端截止或 Ctrl+C 不能撤销上游可能完成的发送。pending/unknown 只能由用户核对后另作处理，首版不提供强制重发。
成功返回 outcome=accepted、messageId、deduplicated，不自动轮询验证；需要时单独用 message get 回读。
上游消息短 ID 依赖映射，不能把 get_msg 失败当作未发出；本地记录也不构成服务端 exactly-once 保证。
跨设备、不同 endpoint 别名、删除记录或调用方使用新请求键都不受本地去重保护。

## 实现和验收顺序

先完成命令/配置/传输/只读接口，再做账号门禁与发送记录，最后开放单次文本发送。
用合成响应及本地 HTTP 假服务验证参数、鉴权、协议、账户门禁、超时和未知发送语义；不使用真实聊天夹具。
通过子进程测试 stdout JSON、stderr 脱敏、退出码和两个 CLI 同键并发仅一次网络发送。
以 cargo fmt --check、cargo clippy --all-targets --locked -- -D warnings、cargo test --locked 和 cargo build --release --locked 验证。
检查原生交付文件与 Cargo 源码包清单，确保不包含实验材料、NapCat、凭据或私有上下文。
交付预期为原生 CLI、用户文档、本项目许可及必要依赖通知；不承诺所有目标静态链接或无系统依赖。
本轮不决定公开发布、不复用上游许可文本。
真实回归必须另有明确账号、会话和测试内容；过去的单次发送授权不扩展为自动集成测试授权。

## 依据

NapCat 基线为 v4.18.38，提交 bbd0d2aca3d470c20b33b1d4f95123335633bf26。
API 映射来自固定产物与以下源码的静态核对，并结合已有实验，不复制上游实现：
[私聊历史](https://github.com/NapNeko/NapCatQQ/blob/bbd0d2aca3d470c20b33b1d4f95123335633bf26/packages/napcat-onebot/action/go-cqhttp/GetFriendMsgHistory.ts)、
[消息读取](https://github.com/NapNeko/NapCatQQ/blob/bbd0d2aca3d470c20b33b1d4f95123335633bf26/packages/napcat-onebot/action/msg/GetMsg.ts)、
[私聊发送](https://github.com/NapNeko/NapCatQQ/blob/bbd0d2aca3d470c20b33b1d4f95123335633bf26/packages/napcat-onebot/action/msg/SendPrivateMsg.ts)。
Rust 路线依据为 [Cargo manifest](https://doc.rust-lang.org/cargo/reference/manifest.html)、
[clap](https://docs.rs/clap/latest/clap/) 和 [reqwest blocking ClientBuilder](https://docs.rs/reqwest/latest/reqwest/blocking/struct.ClientBuilder.html)，
查阅日期 2026-10-17，在线 latest 为可变来源，不代替实现时的锁文件和构建验证。
官方在线 API 文档是可变参考，不覆盖固定产物核对。上游许可与个人账号风险仍未因 HTTP 包装而消除。
