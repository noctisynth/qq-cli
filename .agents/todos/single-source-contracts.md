# 单源契约整改实施

文档编号：TASK-004。状态：Active。更新：2026-10-21。
关联：RFC-0006、SPEC-004、PLAN-001、TASK-001、TASK-002、CHECK-049。

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
- [ ] 输入移除参数名特判：从Clap取类型/默认/依赖；共享字段新类型/约束供解析与Schema使用，集中不可静态表达的规则。
- [ ] 删除所有迁移桥、手写输出Schema与Schema-only副本，新增结构门禁防回退；逐项核对34命令/45帮助/六Skill。
- [ ] 最终离线构建、契约/负向/包/变更集检查与阶段证据汇总，不混同真实QQ或稳定性全部交付。

当前：普通JSON与NDJSON输出切片已验收，CHECK-050至CHECK-056；下一步Clap输入统一及最终结构/离线门禁。业务68/100的含义保持，本整改尚未全部完成。

阶段二：2026-10-21 契约整改阶段二：status/doctor及start/restart/update/rollback/stop/remove已接入实际派生类型，删除对应手写Schema。CHECK-051记录42项定向离线测试；下一步资源发现与分页切片。业务68/100不变，整体迁移未完成。

阶段三：资源发现七命令及集合分页已从实际DTO派生Schema，完成119项定向离线回归；CHECK-052。下一步消息读取/历史/导出递归契约，业务68/100不变。

阶段四：消息get/history/search/export及递归转发/媒体读取已使用实际Rust类型；CHECK-053。下一步写操作/附件和自省，再完成事件及输入统一；业务68/100保持，整体迁移未完成。

阶段五：发送/回复/请求查询、附件指纹、下载和清理实际输出及错误details/nextAction已类型化；CHECK-054。下一步Skill/自省及事件、输入和最终桥清理；业务68/100不变。

阶段六：schema/skill五命令从实际DTO派生契约，无通用响应兜底；LegacyWire/success(Value)/旧分页桥已删除。作用域为资源、消息、写入和已确认状态的必需字段；CHECK-055。后续事件NDJSON和Clap输入统一、结构门禁及整体验收。

阶段七：事件NDJSON使用实际Event/Notice/Recovery类型，删除最后手写输出Schema；30项事件/CLI/输出回归与最终7项事件契约测试通过，CHECK-056。剩余输入统一、结构门禁及最终离线整体验收。
