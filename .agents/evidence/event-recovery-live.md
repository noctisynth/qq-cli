# 事件检查点、通知与有界恢复

文档编号：CHECK-048。状态：有界恢复开发及正常功能验收通过。更新：2026-10-20。
关联：PLAN-001、SPEC-004、RFC-0003、TASK-002、CHECK-030、CHECK-044、CHECK-047。基线573b845；用户许可仅两个已登录实验账号间合成文字及一次测试撤回。私人目标/正文/请求键/凭据与运行输出只留忽略目录，不扩展其它会话操作。

缺口：原watch已有有限重连、gap和本次进程内去重，没有检查点或历史补读。新增events/checkpoint与watch/recovery模块，不复用export检查点。w1由qq-cli本地原签名权威生成，schema1绑定scope/run/conversation及恢复起始秒、最多256条稳定消息hash，无正文；至多24小时，错误签名、错账号/实例/run/会话或时间过期/未来拒绝。不是NapCat可续传事件日志或账号安全证明。

公开event watch --conversation CONVERSATION --resume CHECKPOINT，必须显式会话。指定会话ready/数据/summary携带检查点；订阅并复核身份之后才查询既有history内核的一次有界结果，沿原limit/累计字节/截止并预留引用开销。正常指定会话重连同样补读。历史与实时以相同签名Locator的SHA-256作稳定eventId，source区分history/live；summary.recovery报告次数、扫描/发出量、起点、历史边界及stopReason。历史即使complete也不能恢复通知或证明连续，gap仍recovered:false/coverage:unknown，不假报补齐；256标识窗口外交给宿主继续按稳定ID去重，不承诺exactly-once。输出失败停止，恢复查询失败有summary，错检查点在WS/领域读取前拒绝。

真实：两个定向私聊各监听120秒并互发一条合成文字，各发送仅一次accepted且cached:false。两端均观察incoming/outgoing各1条，零重复/缺口，正常deadline结束。用户在发送方官方窗口撤回一条，接收端friend_recall通知supported:true，userId及接收账号本地messageId与先前incoming消息相符。最初直接将发送端回执ID与接收端撤回ID比较不相等；改按各自作用域定位后吻合，不将跨账号局部ID差异误判为后端错误。发送方自己撤回不据此承诺会话定位或上报。

监听结束后再由许可账号发一条合成文字、使用接收端summary检查点resume12秒：ready/gap/history消息/summary，消息仅1条且正文匹配合成输入；duplicates2，gaps1，recovery扫描13/发出1/backend_end/historyComplete:true。扫描是底层访问，不是对外输出13条旧正文；仍只声明backend_accessible。将同token用于另一实例返回EVENT_CHECKPOINT_STALE，无新发送。三条发送意图均独立固定键，不重试未知结果。

验证：CLI12、event_cli1、event_schema1、events6、watch6（含显式恢复/默认重连补读/跨会话前置拒绝）、检查点绑定/过期/签名/256上限1及Skill2，共29项唯一用例通过，重跑不重复计数。fmt/clippy/release、架构/219文件源码包通过。编译初期chrono未启用clock，改用现有SystemTime→UTC转换；clippy折叠分支修正后通过。原生缓存硬链接与许可元数据警告保留。无NapCat修改/新加载器。

证据层级：friend_recall真实通过，其余支持的通知种类/未知子型仍以6项归一化合成为证据；重连用受检原创服务/Source合成路径，真实故障注入/长断线/稳定性不提升。本阶段完成原检查点/去重/补读功能项，业务原混合清单67→68/100（整体107含5基础+2设计，不把数量当工作百分比）。完整登录挑战、全部生产通知和交付仍未完成。接着按用户顺序检查帮助/命令Schema/Skill一致。
