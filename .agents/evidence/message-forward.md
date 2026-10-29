# 单条及合并转发

文档编号：CHECK-147。状态：引用转发切片离线通过。日期：2026-10-29。
关联：RFC-0009、SPEC-009、TASK-007、CHECK-146。

依据固定NapCat v4.18.38/bbd0d2aca3d470c20b33b1d4f95123335633bf26的ForwardSingleMsg、SendForwardMsg、SendMsg及types/message来源。message forward支持1至32个m1引用，单条默认普通转发，多条或merged为合并卡片；首次意图内逐条核对账号/实例/运行及消息身份，源和目标都授权。实际序列化为真实节点ID，空nickname/content只满足声明字段，ID分支使用实际消息，不制造发送者。引用节点子集不冒充完整原生混合Schema。

跨会话需allow-self-copy，并授权自己的私聊目标；源非Packet路径可能先复制到自己，预览/回执标maySendSelfCopies。所有来源仍须同账号/实例/运行；单次业务请求不等于源复合操作原子化。源会跳过部分无法获取的节点，引用数只是提交数量；接受不证明每条内容或最终送达。
单条源返回null，不伪造新消息ID；合并源消息ID经现有规范验证后，只保存messageId/conversation最小元信息，供缓存及request get恢复。原始参数、消息正文、res_id和完整原生结果不保存；缺/非法ID、输出超限为unknown且不重发，缓存回执不符目标拒绝。旧普通API和撤回/已读绑定摘要保持原形，不借元信息扩展降级来源。

两个原创测试覆盖前置范围/预算及十二种受管分支：单条群/私聊、合并群/私聊、显式单节点合并、跨会话缺许可/允许/自己权限拒绝、接收拒绝、异常ID未知、业务拒绝和来源变化；额外核对同键绑定变化、缓存/离线查询一致及损坏目标回执拒绝。原始正文/原生秘密不落状态，实际输出匹配派生Schema。首版未知退出码期望与既有5不符，修正夹具后通过；API六项来源/故障单测、API/CLI/Skill及撤回/已读、输入/公共契约回归通过。all-targets Clippy、结构/契约、Skill quick_validate、源码包白名单469文件、fmt/diff通过。

未执行真实QQ或macOS新链路验收；完整混合消息参数Schema、自定义节点及其它能力组仍待完成。Windows x86_64累计release构建通过，发布二进制的forward Schema与离线全局已读预览冒烟通过；不以离线回执代替实测。
