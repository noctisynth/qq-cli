# 登录观察实现验收

文档编号：CHECK-019。状态：协议内核离线通过，受检传输及CLI待接。更新：2026-10-19。
关联：CHECK-016、SPEC-004、RFC-0003、TASK-002。

napcat/control实现固定POST路由、SHA-256(token+.napcat)管理鉴权及CheckLoginStatus；Backend要求在发任何凭据前证明同socket归属。观察最多两次调用共用截止，2FA/拒绝/异常立即结束，不触发QQ登录、刷新、密码、快速登录或设备接口。64KiB响应预算；Credential私有且不实现Debug/Serialize，不持久。状态只输出固定八种阶段（含unknown）及四个布尔提示，无账号/二维码URL/错误正文；阶段不决定账号ready，不一致布尔拒绝。

4项原创纯协议测试覆盖固定独立hash向量、七阶段及未知字段脱敏、初始化阶段与在线提示独立、2FA/拒绝/坏凭据零后续请求、严格字段/类型/重复code、冲突布尔、异常JSON/字节预算/传输失败及过期截止不重试。cargo test --offline --locked -j1 --lib napcat::control全部通过，全目标clippy -D warnings通过；fmt/架构/源码包检查见补记。此次没有新增网络实现或真实QQ操作，不代表公开status已提供控制阶段或原版登录恢复完成。此前完整220项证据仍见CHECK-018，新模块增加4项，尚不称完整224项回归已通过。

提交前fmt、架构检查和167文件源码包检查通过，Semifold status识别34个changeset、仍计划0.1.0到0.2.0，不消费或发布。包许可元数据既有警告保留。
