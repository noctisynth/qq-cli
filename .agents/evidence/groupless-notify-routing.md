# 无群通知哨兵导致事件解析失败的修复

文档编号：CHECK-062。状态：已定位并修复，定向验收通过。日期：2026-10-21。
关联：SPEC-004、TASK-001、TASK-002、CHECK-060、CHECK-061。

本轮用户授权继续排查两问题，可修复本地实现并同步提交；真实操作仅受检只读观察和指定实验群/账号角色读取，没有新增发信、撤回、权限变更、启动或停止QQ。

受检私人观察器保留签名run/Job/进程/连接归属和实际身份检查，只输出错误帧类型与字段结构，不保留状态正文、非测试对象标识或凭据。实际第2帧捕获BACKEND_PROTOCOL，结构post_type=notice、notice_type=notify、sub_type=input_status、group_id为数字0；无消息段和message_id。NapCat固定v4.18.38提交bbd0d2aca3d470c20b33b1d4f95123335633bf26的packages/napcat-onebot/event/notice/OB11InputStatusEvent.ts明确声明group_id=0，与实际结构相符。原代码conversation先对所有存在的group_id调用正整数校验，错误地拒绝该合法无群哨兵，导致整个watch失败。

修复：仅notice/notify的数字或字符串零群哨兵路由为无会话上下文。显式会话及目标策略继续过滤；无过滤订阅仅产生supported:false/conversation:null，不透传status_text。已知friend/group通知和负数、空值等校验不放宽，不构造group:0，不伪造私聊目标。输入状态仍不属于新增支持通知类型，公开响应类型和Schema不变。

新增两项回归覆盖两种零值、无过滤/群过滤/私聊过滤/权限目标列表及后续正常消息可继续，零群已知群通知与负数/null/空串notify仍BACKEND_PROTOCOL。最初新增测试误要求可空conversation字段省略，改为现有Serialize/JsonSchema的null约定，未改响应契约。events8、event_schema1、watch6共15项通过。

更新后release CLI在原指定群只读监听90秒，ready后正常deadline/退出0，filtered9、emitted0、gaps0、无错误。此复核不声称期间再次实际出现同一输入状态；确定性触发覆盖来自实际捕获结构构造的匿名合成用例。CHECK-060历史首帧未保留，不能证明历史那次只有此一原因；本轮确认并修复实际复现的该原因，不保证所有BACKEND_PROTOCOL均应忽略。

fmt、全目标Clippy、release、架构与单源契约门禁及Semifold config/status通过，patch:fix登记不消费。首次沙箱Clippy受构建资源/进程失败影响，正常权限-j1复核通过；保留增量硬链接及页面文件警告，不修改检查门槛。私人额外原解析器观察未作为新触发已发生证据。角色缓存证据随后单列；稳定性暂缓，业务混合计数68/100保持。
