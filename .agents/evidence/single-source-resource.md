# 资源发现契约整改

文档编号：CHECK-052。状态：切片通过。日期：2026-10-21。
关联：TASK-004、RFC-0006；基线75e9d30。

contact/list/get、group/list/get/members、conversation/list/get由实际Contact/Group/Member/Recent生成Schema。规范化、筛选、排序和成员校验访问类型字段；Row仅作封闭分派，不含JSON回退。集合分页使用结束/续页枚举，前者游标为null，后者必须有c1游标；实际Paged容器派生对应契约。删除旧read_response手写定义。补齐无实例status的Pending类型出口，前阶段该边缘分支遗漏的JSON桥已移除。

119项定向离线测试通过，覆盖库内合成测试、collections/public_contracts/writes/send_cli；实际资源结果与派生Schema一致，包含错误范围、游标、数值账号、额外字段与筛选职责负例。fmt、架构及全目标Clippy通过。未操作真实会话；整体迁移与输入/事件/错误分页完全统一仍未完成。上一阶段诊断测试的格式规范化一并纳入。
