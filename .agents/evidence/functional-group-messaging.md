# 指定群正常功能验收

文档编号：CHECK-038。状态：指定群文本/回复与读取功能通过。更新：2026-10-19。
关联：CHECK-037、SPEC-004、TASK-002。CLI基线3bb3b5c，固定Framework v4.18.38与同一签名实验实例；用户明确提供测试群，并另行确认两条固定测试文本、各一次且不提及成员。真实群ID、账号、正文、引用、截图只留忽略目录，不进入公共夹具。

group get成功；group members首批5人，再只带cursor续页得5人，10个唯一userId、complete:true，与群信息memberCount:10一致。未读取其他群或扩为全量联系人查询。最初message history空列表、backend_end仅表明当时可访问范围；发送后2条、用户补发后3条，不从最初为空推导该群从未有消息。

message send及对其message reply各调用一次，两个独立固定请求键，均accepted/cached:false。历史中回执ID各仅一条，对应text及reply+text。message get读回复，再通过其reply.messageRef读回原消息，目标ID与首次发送回执一致。用户提供群窗口截图，显示一次原消息、一次正确引用回复及用户补发消息，支持窗口/API正常对照，不据截图推断长期焦点。

第一段60秒event watch捕获1个outgoing，第二段90秒捕获1个incoming，均ID匹配最终历史，summary为deadline，duplicates:0/gaps:0。第一段未覆盖回复发送时间，不声称同时取得两条outgoing或完整投递保证。message search测试关键词返回该3条且backend_end；message export新建输出3条，ID顺序与历史一致、complete:true。正文仅保存在忽略目录，未重发或用未知结果换键。

本阶段未发现业务代码缺陷，不需要修复。group list全量发现、群提及、群附件及群转发不由此验收；跨进程/断线/长期运行仍按用户要求暂缓。用户文档记录新增可用范围，使用patch:chore changeset；混合任务67/100保持，不勾选全部群/联系人发现或完整写入门槛。
