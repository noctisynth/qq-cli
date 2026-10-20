# 写操作及错误类型整改

文档编号：CHECK-054。状态：切片通过。日期：2026-10-21。
关联：TASK-004、RFC-0006；基线a24d900。

send/reply使用实际Accepted/DryRun与Receipt类型；Snapshot/Retained复用Fingerprint，Kind/Outcome/StartOutcome由共同定义重导出。账本JSON仍是私有持久化兼容边界，读取后按既有校验转入明确公共RecordedReceipt枚举，不将任意JSON穿透到输出。request.get启动与消息请求分支、media.download/prune实际结果均派生Schema；删除对应手写定义。

AppError实际details与nextAction改用封闭Details/NextAction，错误outcome复用实际枚举；所有生产调用点按字段构造。包含候选实例、安装引用、身份变更、登录待确认和兼容性证据等既有脱敏提示，不增添真实操作。传输原始JSON及私有账本未冒充公共DTO。

fmt、架构及全目标Clippy通过；完整串行离线回归313项通过。编译阶段更新合成测试的字段访问与类型断言，保持检查内容；未连接实际QQ。Skill/自省、事件及输入单源整改仍未完成。
