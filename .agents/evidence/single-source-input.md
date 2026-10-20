# 输入契约单源整改

文档编号：CHECK-057。状态：切片通过。日期：2026-10-21。
关联：TASK-004、RFC-0006；基线aa5c6cd。

Cli参数采用实际LocalId、DecimalId、Bounded/Limit/Seconds、Text、Rfc3339、Handle、NonEmpty及Conversation；Clap通过FromStr解析，Schema适配依据解析器TypeId从同一类型派生，不以参数名分支推测类型。默认/必需/标志/候选值继续从Clap读取，布尔候选保持JSON布尔而非字符串；未知解析类型明确失败。Conversation字符串序列化有同类型Schema实现，不复制CLI字段结构。

同一rules规则表既验证实际Args序列化投影，也投影JSON Schema跨参数约束：必选、条件依赖、互斥、提及及附件组合预算；冲突从Clap读取。附件、提及和文本预算常量供运行校验与Schema使用。时间先后及账号/目标/文件/签名等运行语义仍由原有专用验证负责，不宣称标准JSON Schema可表达所有状态条件。maxUtf8Bytes是UTF-8字节预算注解；普通字符长度/format不替代字节和时序验证。

43项CLI/输入矩阵/内容/发送/集合/历史/Skill/公共契约测试通过，另参数重命名保留类型约束的定向库测试通过；Clippy、fmt与架构门禁通过。矩阵覆盖有效与非法ID、数值范围、布尔创建依赖、游标替代、空搜索、日期、互斥正文及17附件预算。未操作真实QQ。

验收发现并修复：From<&str>被Clap优先选择导致非法参数panic，已删除便利转换，仅保留可失败FromStr；日期非法字面值现在可在解析阶段正常拒绝，仍退出2且不读取配置/输入文件。修复将Clap布尔possible_values误投影为字符串enum的问题。语法约束前移测试接受解析或语义阶段拒绝，不放宽拒绝条件。
