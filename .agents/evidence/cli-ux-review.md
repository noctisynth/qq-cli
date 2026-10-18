# CLI 接口整体审视与职责修订核对

文档编号：CHECK-012。状态：设计检查完成，未实现或实测调用成本。更新：2026-10-18。
关联：RFC-0004、RFC-0003、PLAN-001、SPEC-004、TODO-001、TASK-001、TASK-002。

## 2026-10-17 收敛观察（历史）

状态/身份/登录/能力有多个重叠入口，改为 status、doctor 与幂等 napcat start，各有单一职责。
回复、导出、转发展开和媒体元数据回收进 send/history/search/get；查找统一 list --query，详情统一 get。
统一 skill/schema/request get、event 单数、limit/timeout/cursor，取消公开后端页数和输出投影语法。
forget 改为 remove，安装仍 install/installations/uninstall，不更改安装/实例区分及引用保护。
保留全部已确定业务能力，取消每任务必经 schema/doctor/dry-run，成功结果只给必要任务/作用域/分页数据。

另修正两处准确性问题：满足页 limit 应成功而非部分失败；start --new 需持久请求键而非仅用进程锁防重。
续传与 reply-to 引用推导作用域，显式参数冲突拒绝，不能降低身份与 unknown/gap/覆盖范围的可见性。
流作用域在 ready 给出，数据事件不重复整份包装，重连不默默切号。

## 2026-10-18 按用户澄清修订

以语义和职责清晰为先，不以命令最少为目标。contact list/get 管好友，group list/get/members 管群和成员，
conversation list/get 只管最近会话和会话元数据；--query 不再隐式把最近聊天扩大为全好友/群发现。
恢复独立 message reply 和 message export，前者推导引用目标，后者负责文件/覆盖/检查点/续写。
send/reply 共用载荷校验和防重，history/search/export 共用查询机制；公共入口不因实现共享而强制合并。
保留默认实例选择、合理预算、紧凑输出、正常分页成功、未知结果语义及 start --new 请求键。
PLAN-001、SPEC-004、RFC-0004、DESIGN-001 与全部 TASK/TODO 同步，示例不再使用 send --reply-to 或 history/search --output。
历史“30个叶子命令”的聊天说明已不是当前树，不能用它限制更新后的接口。

## 检查

检查命令迁移表与目标方案/Spec/RFC/TASK 一致，旧入口不再出现在目标命令示例，原 Rust 骨架保留历史事实。
检查文档唯一编号/引用、Markdown 链接、JSON 示例可解析、隐私、diff 格式及 Semifold 配置/status。
工程基础5项和设计2项保留，扩展实现/交付0/35，不把设计合并视作业务完成。
确认暂存区为空，保留已有未提交文档变更，不执行 git add/commit/version/publish。
无代码/依赖变更，不重复工程测试，无 QQ/NapCat 启停、登录、读私聊或发送操作。

这些是静态接口结构改进，不声称测得 token 降幅、端到端调用成功率或真实兼容性。
实际命令迁移与两个 Agent 宿主验收仍在 TASK-002；命名/职责以本记录的最新修订、PLAN-001 和 RFC-0004 为准。
