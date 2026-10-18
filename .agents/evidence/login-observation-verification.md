# 登录观察实现验收

文档编号：CHECK-019。状态：协议/受检HTTP及只读恢复离线通过，CLI待接。更新：2026-10-19。
关联：CHECK-016、SPEC-004、RFC-0003、TASK-002。

napcat/control实现固定POST路由、SHA-256(token+.napcat)管理鉴权及CheckLoginStatus；Backend要求在发任何凭据前证明同socket归属。观察最多两次调用共用截止，2FA/拒绝/异常立即结束，不触发QQ登录、刷新、密码、快速登录或设备接口。64KiB响应预算；Credential私有且不实现Debug/Serialize，不持久。状态只输出固定八种阶段（含unknown）及四个布尔提示，无账号/二维码URL/错误正文；阶段不决定账号ready，不一致布尔拒绝。

4项原创纯协议测试覆盖固定独立hash向量、七阶段及未知字段脱敏、初始化阶段与在线提示独立、2FA/拒绝/坏凭据零后续请求、严格字段/类型/重复code、冲突布尔、异常JSON/字节预算/传输失败及过期截止不重试。cargo test --offline --locked -j1 --lib napcat::control全部通过，全目标clippy -D warnings通过；fmt/架构/源码包检查见补记。此次没有新增网络实现或真实QQ操作，不代表公开status已提供控制阶段或原版登录恢复完成。此前完整220项证据仍见CHECK-018，新模块增加4项，尚不称完整224项回归已通过。

提交前fmt、架构检查和167文件源码包检查通过，Semifold status识别34个changeset、仍计划0.1.0到0.2.0，不消费或发布。包许可元数据既有警告保留。

## 受检HTTP增量

共享transport/owned_wire私有hyper POST及原socket的Job/Pinned验证，OneBot包装接口保留。napcat/control/owned实现固定路由和请求形状检查，鉴权只传hash、不带Bearer，状态只传服务器Credential和空body；每次连接独立认证归属，64KiB上限与更低配置预算生效。无重定向、重试、代理或query凭据，不改变身份/登记状态。

8项owned_transports测试通过，包含新增2项管理测试：IPv4/IPv6正常鉴权及脱敏观察、2FA/拒绝/重定向只发一次、超大状态拒绝；错Job/端点零应用字节，过期截止及非法请求形状零请求。原OneBot固定/分块/媒体/错误预算和WS及签名CLI测试保持通过。最终完整226项离线回归、全目标clippy、fmt、release、架构和169文件源码包检查通过，编号/隐私及Git diff检查通过。Semifold36个changeset，计划版本不变，不消费或发布。缓存硬链接回退及包许可元数据既有警告保留。未调用真实QQ/NapCat，控制端点仍由测试给定，公开CLI恢复未接。

## 原控制能力及秘密只读恢复

run_job/control仅按当前完整实例/安装快照重开签名j1的原prelogin进程/Job，核对映像/固定端点及已有API同进程/端点；必须有原t1保留凭据，不连接、不扫描、不再次retain、不修补旧日志。runtime_store/recovery独立于verify_prepared，只读原r1/最终marker和原vault，核对计划、端点、版本及秘密引用，不要求live日志/缓存为空，不读取其正文。返回管理材料不实现Debug/Serialize，不创建状态、秘密或目录。两能力尚未在应用层组合，因此不把秘密存在当作进程权威。

11项runtime_store定向测试通过：新增1项覆盖运行后日志/缓存容忍、原管理token区别API token、换run/marker/安装拒绝及缺秘密不补建；扩展原父子预登录测试核对只读控制重开与丢t1拒绝，保持无新增保留日志。最终完整227项离线回归、全目标clippy、fmt、release、架构及171文件源码包通过。Semifold37个changeset，计划版本不变，不消费或发布；既有警告保留。公开status、真实原版及完整登录恢复均未验收。
