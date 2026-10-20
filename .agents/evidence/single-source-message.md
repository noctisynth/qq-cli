# 消息读取单源契约

文档编号：CHECK-053。状态：切片通过。日期：2026-10-21。
关联：TASK-004、RFC-0006；基线a1c78cd。

实际Message/Sender/Segment/Node/Media规范化结果派生Schema；递归转发展开操作类型字段，不再构建输出JSON。内部resource_id跳过序列化。未知段仅输出受控类型名及supported:false。可下载媒体必须携带引用；不可下载媒体允许保留不可下载引用。消息引用、ID、会话与时间保留约束。

历史页使用结束/可续读分支、共享统计，派生Schema与实际对象一致；PageReply统一成功、部分结果及失败，部分结果保持退出7。导出实际Export类型共享历史页，同时保留已完成导出resume不返回pageInfo的分支；正文NDJSON内容仍复用实际Message序列化。删除旧消息与历史手写Schema。事件手写外壳暂时引用派生消息的defs，后续整体替换。公共错误细节及事件摘要的迁移桥仍待清理。

41项消息/历史/媒体CLI/watch/eventCLI定向合成测试与97项库内测试通过，覆盖TLS媒体、转发循环/预算/引用、分页、搜索、原子导出和恢复补读；最后严格字段注解追加后，消息/历史/collections/public_contracts共30项重新通过，含实际导出Schema与非法格式负例。fmt、全目标Clippy和架构检查通过。没有操作真实QQ；源编译阶段修复借用及类型调用差异，不放宽实际协议校验。
