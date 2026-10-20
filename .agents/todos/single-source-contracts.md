# 单源契约整改实施

文档编号：TASK-004。状态：Active。更新：2026-10-21。
关联：RFC-0006、SPEC-004、PLAN-001、TASK-001、TASK-002、CHECK-049。

- [x] 同步用户确定的方向、源边界、兼容要求及分批清单；仅设计，不计实现。
- [x] 固定schemars依赖/许可/序列化生成设置，建立共享基础及封闭标识类型，明确过渡桥和最终移除门禁。
- [x] 安装管理完整切片：install/installations/uninstall/instances实际DTO及Schema同源，真实输出形状离线验收。
- [x] 诊断/登录/实例维护切片：status/doctor/start/stop/restart/remove/update/rollback公共类型。
- [x] 资源发现切片：contact/group/conversation及分页类型、字段约束。
- [ ] 消息读取/历史/search/export、递归转发/媒体切片，去除公共任意Value。
- [ ] send/reply/request及附件/下载/prune切片，accepted/unknown/dry-run与各类结果精确建模。
- [ ] NDJSON ready/message/notice/gap/summary使用实际事件枚举及派生Schema，保留检查点与恢复预算。
- [ ] 错误/details/nextAction、分页/部分结果与Envelope完全类型化；自省schema/skill使用明确类型，避免剩余通用envelope。
- [ ] 输入移除参数名特判：从Clap取类型/默认/依赖；共享字段新类型/约束供解析与Schema使用，集中不可静态表达的规则。
- [ ] 删除所有迁移桥、手写输出Schema与Schema-only副本，新增结构门禁防回退；逐项核对34命令/45帮助/六Skill。
- [ ] 最终离线构建、契约/负向/包/变更集检查与阶段证据汇总，不混同真实QQ或稳定性全部交付。

当前：基础与安装切片已验收，见CHECK-050；剩余切片未验收。下一步资源发现与分页切片。每完成切片拆分状态，不用总大项隐藏进度；如需细分不改变既有业务68/100的历史含义。仅本项目原创代码/依赖/文档与合成测试获授权，不新增真实会话操作。

阶段二：2026-10-21 契约整改阶段二：status/doctor及start/restart/update/rollback/stop/remove已接入实际派生类型，删除对应手写Schema。CHECK-051记录42项定向离线测试；下一步资源发现与分页切片。业务68/100不变，整体迁移未完成。

阶段三：资源发现七命令及集合分页已从实际DTO派生Schema，完成119项定向离线回归；CHECK-052。下一步消息读取/历史/导出递归契约，业务68/100不变。
