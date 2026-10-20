# 自省及响应桥清理

文档编号：CHECK-055。状态：切片通过。日期：2026-10-21。
关联：TASK-004、RFC-0006；基线68c8845。

schema list/get和skill list/get/export返回实际CommandSummary/Description/SkillSummary/SkillGuide/SkillExport。Description的SchemaDocument明确封装schemars::Schema，开放JSON Schema文档是受控协议边界，不是领域数据兜底；注册表只映射实际类型，未知遗漏返回错误。离线消费者的get序列化视图复用describe，不维护另一份字段对象。

删除LegacyWire、success(Value)、旧paged/partial Value接口。Success泛型元数据区分OptionalScope/Scoped，集合/历史、读取、写入、已确认状态及启动结果必需scope，未确认状态和离线自省仍不冒充身份。共享错误省略字段不允许显式null。

早期21项Skill/CLI/自省测试通过，最后作用域容器变更后重跑相关集合、历史、写入、请求、CLI/Skill及公共契约测试；54项通过；Clippy、fmt及架构门禁通过。没有操作真实QQ。事件及输入整改仍未完成。
