# 群精华、待办与签到

文档编号：CHECK-151。状态：R08首段八action离线通过。日期：2026-10-29。
关联：RFC-0009、SPEC-009、TASK-007、CHECK-150。

固定NapCat v4.18.38/bbd0d2aca3d470c20b33b1d4f95123335633bf26的group/SetEssenceMsg.ts、DelEssenceMsg.ts、core/apis/group.ts、packet/BaseGroupTodoAction.ts及Set/Complete/CancelGroupTodo、extends/SetGroupSign.ts、GetGroupSignedList.ts已核对。message essence显式enabled真假，绑定签名群消息且只接受缓存短ID；message todo显式open/complete/cancel，从同一定位取原生序列并在首次意图内重读核对。group sign显式一次签到，send_group_sign为同处理器兼容直接入口；group signed有界投影首批。八项协议DTO派生Schema，领域仍严格类型。

原生todo在有message_seq时不读message_id，缺时可能从另一个会话的缓存取seq；raw源检查按实际优先级决定，受限目标下有源读取的路径拒绝。精华删除的完整seq/random/group元组优先于message_id，纯元组可按实际群授权；不完整元组不能靠伪造group_id隐藏来源。签到与待办把群号转JS数字，复用明确的安全整数检查，原戳逻辑移至同一核心标识校验；拒绝舍入目标。签到源只取page[0]，complete:false；rank保留小数，时间单位未确认，用nativeTime保持数值，不伪造成秒或完整今日名单。

原创受管CLI十七分支覆盖精华设置/移出、待办三态及长消息ID按seq操作、签到/首批列表、不精确返回QQ号、来源序列变化、私聊/缺seq/长精华ID、读权限/目标拒绝及待办来源读权限拒绝。成功写同键缓存不联网；输出通过派生Schema，状态无源正文或原生扩展。离线预览、显式参数/数字范围、API十项单测中的原生优先级与来源策略，以及CLI14/Skill3/截止/输入和互动/卡片/群查询管理/撤回转发回归通过。

all-targets Clippy、结构/契约、Skill quick_validate、源码包白名单479文件、fmt/diff通过；Semifold只规划14份变更集，未发布。前一R07累计Windows release构建及91叶命令/按钮Schema冒烟通过，该产物不冒充本次R08发布构建。

公告三项与精华列表仍待开发；未执行真实QQ、Packet或macOS新链路验收。OneBot接受不证明精华/待办/签到效果，原稳定性和完整功能门槛仍在TASK-007及原专题维护。
