# 实例和诊断单源契约

文档编号：CHECK-051。状态：切片通过。日期：2026-10-21。
关联：TASK-004、RFC-0006、SPEC-004。基线2a6e5fb。

status及start/restart/update/rollback/stop/remove实际业务输出使用封闭状态和DTO；登录观察类型迁至core契约并由原协议路径重导出，未保留副本。doctor使用诊断检查枚举，Host/Application/Verdict移至共同实际类型，原入口重导出；未输出被serde跳过的本地可执行路径。删除旧status/start/stop/remove手写Schema，注册仅路由实际类型。doctor消除通用响应兜底，剩余通用响应6项。

38项实例/CLI/启动/维护/契约定向测试通过，另4项doctor合成诊断通过；实际序列化产物对照派生Schema，未知状态/额外字段被拒绝。架构、fmt与Clippy检查通过。没有启动、停止或查询真实QQ。登录、身份、授权和所有权逻辑不变，保留原JSON字段与退出行为。

迁移中共享Success仍允许可选scope；最终强作用域容器、错误细节、分页与事件由TASK-004后续切片统一，不能据本阶段宣称全部契约收敛。工作区曾出现来源未明的事件恢复删除但留下调用；用户说明没有其他Agent协作。已恢复基线行为，没有保留该差异，不归因于其他修改者。
