# 消息导出功能性对照

文档编号：CHECK-035。状态：授权私聊正常导出功能通过。更新：2026-10-19。
关联：CHECK-031/032/033/034、SPEC-004、TASK-002。CLI代码基线f2eb699，固定Framework v4.18.38及原实验私聊；本轮无新增发信。

message export --limit 2新建忽略目录输出，ok、added/count:2、complete:false且stopReason:limit。以相同output和--resume --limit 10、不重输conversation恢复，added:6、count:8、complete:true/backend_end。输出NDJSON为8条message和1条summary，消息ID顺序无重复并与此前已授权历史一致；逐条对照conversation/sender/time/segments，去除新签发媒体引用后均一致。

完成后再次--resume正常返回added:0、count:8；没有重复追加。另将测试文本关键词与其一秒时间窗口组合导出到新文件，恰有预期单条消息。四次命令标准输出均不含消息正文或segments，仅报告摘要；正文和检查点只在用户指定的忽略目录文件中。输出只代表backend_accessible，不是完整QQ归档。

无业务代码修复。本轮补跑tests/messages.rs共8项通过，含合成嵌套转发展开；未做新的稳定性专项或完整套件回归。真实合并转发样本仍待用户提供，不以合成证据冒充。四部分已通过范围及未完成范围在TASK-002独立列出，不调整原混合67/100计数。文档/changeset/diff与隐私检查及Semifold状态检查后本地提交，不发布。
