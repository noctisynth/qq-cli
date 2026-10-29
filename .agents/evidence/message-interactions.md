# 消息回应、戳与输入状态

文档编号：CHECK-149。状态：R07首段七action离线通过。日期：2026-10-29。
关联：RFC-0009、SPEC-009、TASK-007、CHECK-148。

固定NapCat v4.18.38/bbd0d2aca3d470c20b33b1d4f95123335633bf26的msg/SetMsgEmojiLike.ts、extends/FetchEmojiLike.ts、GetEmojiLikes.ts、SetInputStatus.ts、packet/SendPoke.ts及core/apis/msg.ts、services/NodeIKernelMsgService.ts核对了效应、载荷、返回及限制。七项实际Rust协议DTO派生参数Schema；五条领域命令为message react/reactions、contact poke/input-status、group poke。fetch_emoji_like为同读取业务的底层分页，send_poke为同处理器兼容入口，直接API保留，不额外制造同义命令。

react首次持久意图内重读并核对签名来源，显式enabled真假，拒绝源仅支持短ID时传入长ID。reactions核对来源后以正count调用有界原生分页，长ID只可用签名群来源；原生忽略底层result且可能折为空/部分结果，因此complete:false。tinyId以reactorId输出，不当可用QQ号；未知扩展不投影。戳只用明确peer/target且依赖Packet，源数字转换需拒绝超JS安全整数。裸戳别名按实际group_id而非名称提取会话，私聊不同target_id、不合法类型和舍入标识拒绝；裸消息来源不能凭调用方group_id绕过源限制。输入事件只发送明确数值，不编造未定义状态名/定时任务。原生unknown结果只作为OneBot接受，不保证显示效果。

原创受管CLI十二分支覆盖添加/取消回应、短/长群读取、私聊/群戳、输入事件、上游拒绝、来源时间变化、目标/read拒绝及长ID设置拒绝；预览无业务调用，写键重复不联网，合成状态扫描无源正文/原生扩展。实际输出匹配派生Schema。独立离线检查显式参数及数字范围；七项API单测包含别名目标/舍入/原始来源策略。CLI14、Skill3、输入/截止及好友/群/语音/裸调用适用回归通过。all-targets Clippy、结构/契约、Skill quick_validate、源码包白名单475文件及fmt/diff通过；仅Semifold status规划12份变更集，未发布。

R07另七项及非语音完整格式/新增事件仍待实现；本次未执行真实QQ、实际Packet或macOS互动验收。业务已接受不能替代真实效果，空列表不能断言无人回应。
