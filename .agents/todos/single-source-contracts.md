# 单源契约整改清单

文档编号：TASK-004。状态：Completed。更新：2026-10-24。
需求：RFC-0006。技术：SPEC-007。验证：CHECK-050 至 CHECK-058。

- [x] 同步用户确定的方向、源边界、兼容要求及分批清单；仅设计，不计实现。
- [x] 固定schemars依赖/许可/序列化生成设置，建立共享基础及封闭标识类型，明确过渡桥和最终移除门禁。
- [x] 安装管理完整切片：install/installations/uninstall/instances实际DTO及Schema同源，真实输出形状离线验收。
- [x] 诊断/登录/实例维护切片：status/doctor/start/stop/restart/remove/update/rollback公共类型。
- [x] 资源发现切片：contact/group/conversation及分页类型、字段约束。
- [x] 消息读取/历史/search/export、递归转发/媒体切片，去除公共任意Value。
- [x] send/reply/request及附件/下载/prune切片，accepted/unknown/dry-run与各类结果精确建模。
- [x] NDJSON ready/message/notice/gap/summary使用实际事件枚举及派生Schema，保留检查点与恢复预算。
- [x] 错误details/nextAction使用封闭实际类型，历史及集合分页/部分结果已类型化；CHECK-054。
- [x] Envelope最终作用域约束及自省schema/skill明确类型，移除剩余通用envelope。
- [x] 输入移除参数名特判：从Clap取类型/默认/依赖；共享字段新类型/约束供解析与Schema使用，集中不可静态表达的规则。
- [x] 删除所有迁移桥、手写输出Schema与Schema-only副本，新增结构门禁防回退；逐项核对34命令/45帮助/六Skill。
- [x] 最终离线构建、契约/负向/包/变更集检查与阶段证据汇总，不混同真实QQ或稳定性全部交付。

以上仅为契约整改验收，不替代真实 QQ、平台和长期稳定性验收。
