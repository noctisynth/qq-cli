# 签名消息撤回与显式已读

文档编号：CHECK-146。状态：R06首段离线通过。日期：2026-10-29。
关联：RFC-0009、SPEC-009、TASK-007、CHECK-139。

固定NapCat v4.18.38/bbd0d2aca3d470c20b33b1d4f95123335633bf26的DeleteMsg、MarkMsgAsRead、ForwardSingleMsg、SendMsg、SendForwardMsg及MessageUnique/OneBotAction来源已核对。R06十一项效应为写，七项实际Rust载荷派生Schema；四项混合发送载荷仍parametersKnown:false，不以节点子集冒充完整原生格式Schema。

message recall只收m1签名引用；message mark-read显式会话/引用/全局三选一。引用用于定位整个会话，不声称只读单条。首次写入在共享pending后核对实际身份及来源运行/账号/会话、ID、时间、序号、发送者，拒绝旧运行或不符消息；来源摘要带密钥，记录不含正文。缓存不重新读取撤回后的来源；缺乏绑定的同键裸调用冲突，不重发。全局已读在目标限制下拒绝；普通标记别名依据message_id/user_id/group_id实际优先级，不能按名称推断目标。
裸发送中node/reply/forward及相应CQ来源需要读权限，目标限制下不能只凭接收群证明来源；私有领域Guard不可由参数提供或降级效应。未引入新的运行/unsafe权威。

两个独立CLI测试覆盖离线明确范围和十种受管场景：撤回、引用/私聊/群/全局已读、旧运行、消息变化、来源/读/全局权限拒绝；含多实例引用推导、显式实例冲突、首次准确载荷、缓存不联网、裸键冲突、正文不落状态及实际响应Schema。六项API单测覆盖原有防重及新增pending内来源回调、同键绑定变化/裸丢绑定冲突和来源权限零调用；首版夹具错误码及裸调用缺明确实例已修正，不计产品缺陷。API/CLI/Skill原回归通过；all-targets Clippy、结构/契约、Skill quick_validate、源码包白名单468文件、fmt/diff通过。

未执行真实QQ/macOS新增命令验证；撤回源处理器不检查全部原生结果，回执是上游接受而非最终效果证明。转发领域任务、完整消息格式、事件及扩大故障矩阵继续保留，R06不能整体勾选完成。
