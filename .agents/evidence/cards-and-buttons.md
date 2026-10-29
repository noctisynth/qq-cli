# 卡片生成、消息按钮与复合兼容接口

文档编号：CHECK-150。状态：R07卡片/按钮切片离线通过。日期：2026-10-29。
关联：RFC-0009、SPEC-009、TASK-007、CHECK-149。

固定NapCat v4.18.38/bbd0d2aca3d470c20b33b1d4f95123335633bf26的extends/ShareContact.ts、ClickInlineKeyboardButton.ts、GetMiniAppArk.ts、go-cqhttp/QuickAction.ts，以及core的Buddy/Group服务、miniApp实体与helper来源已核对。四个Ark分享名均只读取/生成：contact card取result=0的arkMsg，group card-json取原生字符串；空内容/原生失败不报可用卡片。严格cardJson字符串只返回本次，不发送或跟随URL。get_mini_app_ark为Packet构造工具，两种模板/自定义字段从实际Rust类型派生Schema，显式API动态结果保真，不为通用Ark对象放宽领域DTO。兼容生成别名及小程序底层工具不重复创建领域命令。

message click签名群来源，原生msg_seq只取绑定序列；私聊、缺序列、来源变化和目标不许可拒绝。首次持久意图内只读核对来源，一次回调，同键缓存不重发；callback正文不落盘。原生unknown返回只当接受，不能证明机器人业务完成。快速复合源立即返回且捕获内部失败，所以.handle_quick_operation归write/源读取，正常确认仍持久unknown，受限目标不信context声明、不重试。完整mixed载荷Schema未冻结，保持parametersKnown:false。

新增原创受管CLI十一分支覆盖两种卡片、原生失败/空内容、按钮正常/来源序列变动/私聊/缺序列/拒绝及卡片拒绝；状态扫描无卡片、回调、源正文和原生扩展。另有上游字符串响应未超限但CLI包装超限的夹具，验证共享任务出口按配置输出预算拒绝。生成名/模板和自定义参数、rawArkData必须字符串均有Schema证据；API九项单测验证别名群优先级、无send权限/无键读取、快速确认unknown持久及同键一次。

CLI14、Skill3、截止及互动/语音适用回归通过；共享预算后账号、联系人、群查询/申请/管理及消息动作/转发、公共契约、普通API回归通过。all-targets Clippy、结构/契约、Skill quick_validate、源码包白名单477文件、fmt/diff通过；仅Semifold status规划13份变更集，未消费或发布。

未执行真实QQ、Packet卡片构造、按钮机器人业务或macOS新链路验收。完整混合消息/Quick参数、后续卡片发送及其它能力组仍按TASK-007继续，不以生成或成功确认当业务交付。
