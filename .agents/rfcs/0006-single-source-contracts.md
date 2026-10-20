# 单源CLI数据契约

文档编号：RFC-0006。状态：Accepted方向，分批落地。更新：2026-10-21。
关联：SPEC-004、PLAN-001、DESIGN-002、TASK-004、CHECK-049。用户明确要求schemars统一实际输出类型及Schema，输入尽量直接取Clap；不创建Schema专用DTO副本，不自研宏框架。

权威：Clap定义命令语法、帮助、默认/必填和可表达的冲突/依赖；领域新类型及共享约束定义字段含义。公共输出DTO同时Serialize+JsonSchema，业务实际构造这些类型，输出Schema使用固定schemars版本及draft2020_12、for_serialize设置生成。序列化契约区别于反序列化默认。跨字段规则优先结构化或移回Clap；无法导出的业务条件集中定义并明确边界，不承诺将权限、在线状态等运行时检查编译成静态Schema。

结构：core内公共契约基础与标识/响应组合，领域DTO按职责拆文件；上层CLI仅路由类型Schema，不反向依赖app。Envelope、分页、错误与下一步也最终类型化；事件是实际序列化的带tag枚举。请求/响应领域对象不能用无约束Value或任意字符串字典替代。上游原始OneBot JSON、JSON Schema文档本身及必要协议适配可保留受控JSON边界，不能直接透传成公共数据。未知消息段按既有安全投影类型保留，不输出原始私密对象。

迁移：保留命令、字段命名、字符串ID、scope、分页/部分结果、退出码及流次序；强化Schema不能放宽已有负面约束。兼容差异须用证据解释。按完整命令切片替换生产DTO构造并删除其手写输出Schema；全局过渡桥必须显式标记、限定入口、跟踪清除，不把序列化转Value的桥当最终架构。不得在不同模块复制相同结构供Schema使用。

验证：同一实际DTO序列化样本通过派生Schema，缺字段/多字段/非法标识与错误变体拒绝；检查JsonSchema与Serde属性、输出省略/nullable、tag、递归及泛型Envelope。现有真实协议不为整改自动重跑；以原创夹具及公开CLI输出契约回归。每阶段同步TASK-004及TASK-001/002、适用changeset并提交。删除最后桥接/手写Schema后才可声明单源整改完成。

依据：Schemars官方derive与SchemaSettings文档，检索2026-10-21，官方latest可变页面不当固定依赖版本；具体Cargo.lock固定版本与许可另随实施证据登记。
https://docs.rs/schemars/latest/schemars/derive.JsonSchema.html
https://docs.rs/schemars/latest/schemars/generate/struct.SchemaSettings.html
