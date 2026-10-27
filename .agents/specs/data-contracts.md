# 类型、Schema 与输入规则技术方案

文档编号：SPEC-007。状态：Active。更新：2026-10-27。
需求：RFC-0006。实施验收：TASK-004。公共行为：PLAN-001、SPEC-004。

## 权威与结构

权威：Clap定义命令语法、帮助、默认/必填和可表达的冲突/依赖；领域新类型及共享约束定义字段含义。公共输出DTO同时Serialize+JsonSchema，业务实际构造这些类型，输出Schema使用固定schemars版本及draft2020_12、for_serialize设置生成。序列化契约区别于反序列化默认。跨字段规则优先结构化或移回Clap；无法导出的业务条件集中定义并明确边界，不承诺将权限、在线状态等运行时检查编译成静态Schema。

结构：core内公共契约基础与标识/响应组合，领域DTO按职责拆文件；上层CLI仅路由类型Schema，不反向依赖app。Envelope、分页、错误与下一步也最终类型化；事件是实际序列化的带tag枚举。请求/响应领域对象不能用无约束Value或任意字符串字典替代。上游原始OneBot JSON、JSON Schema文档本身及必要协议适配可保留受控JSON边界，不能直接透传成公共数据。未知消息段按既有安全投影类型保留，不输出原始私密对象。

契约演进：保留命令、字段命名、字符串ID、scope、分页/部分结果、退出码及流次序；强化Schema不能放宽已有负面约束。兼容差异须用证据解释。各完整命令使用实际DTO派生Schema；手写输出Schema及迁移桥已移除，不重新引入序列化转Value的公共输出桥。不得在不同模块复制相同结构供Schema使用。

## 验证与兼容

真实 DTO 的序列化样本须通过派生 Schema，反例覆盖缺字段、多字段、非法标识及错误变体。
核对 Serde 与 JsonSchema 的省略、nullable、tag、递归、泛型及序列化模式。
集合快照保留既有 JSON 键排序摘要，兼容旧集合游标；不因整改暗改公共字段或退出码。
Clap 提供语法；共享规则提供条件依赖及组合预算；权限、在线身份、文件签名、UTF-8 字节和时间先后仍在运行时核验。
没有通用 data/Value 公共输出桥；JSON Schema 文档、上游协议和私有指纹是受控 JSON 边界。
输出字段真值在实际 DTO，参数真值在实际 Clap 类型，不另建手工字段清单。

依据：固定依赖见 Cargo.lock；schemars 上游可变文档仅解释 API，不作为锁定版本证据：
https://docs.rs/schemars/latest/schemars/derive.JsonSchema.html
https://docs.rs/schemars/latest/schemars/generate/struct.SchemaSettings.html
