# Rust 工程架构与维护入口

文档编号：DESIGN-002。状态：Active。更新：2026-10-22。
产品契约见PLAN-001，重构决定见RFC-0005，质量验收见TASK-003。

## 从任务定位代码

| 子系统 | 职责与入口 |
|---|---|
| app | 组合用例：management路由在administration，实例/引用解析在selection，状态确认及待登录观察在status，启动检查点在runtime，消息调用在messages，事件入口在watch，受管客户端组合在backend |
| cli | args定义命令/参数，validation做无I/O语义校验，registry/schema提供机器契约，skill提供任务指引，stream_output管理进程输出 |
| core | 共享作用域/会话标识、错误、截止和结果信封；不创建进程、凭据或客户端 |
| storage | 配置、本地状态锁/原子落盘、系统凭据和签名引用，不路由命令 |
| requests | 消息意图ledger、跨消息/启动的keys互斥、query离线查询 |
| messaging | 联系人/群/会话、消息/历史/导出、发送、事件用例；attachments封装副本/配额/清理，media封装引用解析、下载与发布 |
| napcat | install封装固定包获取/审计/登记/卸载；compatibility/doctor/qq定义兼容、诊断和QQ安装观察；control封装独立管理鉴权及过滤后的登录观察；runtime封装预留/准备/一次调度/恢复 |
| native | 只读宿主/进程能力、Windows Job及凭据互斥原生边界；unsafe精确限job和credential_lock，后者不处理凭据值或业务逻辑 |
| transport | OneBot动作/响应、受检HTTP、WebSocket和媒体字节封装；不依赖消息用例或应用路由 |

src根层只保留lib.rs和main.rs。lib.rs声明以上边界，main.rs处理参数解析、退出码和标准流。内部Rust路径不是稳定SDK契约；公开CLI契约由PLAN-001和schema界定。

## 应用调用顺序

app入口先做参数语义校验；skill/schema保持无配置I/O。安装固定版本目录检查先于来源、凭据和状态初始化，许可注记仅作信息。管理命令与聊天会话分别路由。

聊天调用由selection选择明确实例或从签名引用推导作用域，保留原有“先拒绝歧义/冲突，再访问相关凭据”的顺序。backend只从签名运行能力构造受管HTTP/WS。status是显式确认实际账号的入口；消息用例不能借此自动承认切号。

启动内核返回Started和运行状态，不构造CLI登录提示。app/runtime把待登录结果转成AUTH_REQUIRED，复用就绪结果共用同一输出函数；账号仍须经过Session核对。具体系统/凭据/API诊断适配器位于app/diagnostics，napcat/doctor只依赖Probe契约，避免生命周期反向调用应用组合层。
status/pending仅组合已有原控制能力、原秘密和固定管理协议；状态事务在网络调用前释放，返回前再核对实例/安装快照。其loginObservation只作进度，不更新账号或ready；原账号确认仍在status的OneBot用例中，消息调用不能借控制面绕过会话门禁。

## 生命周期与传输的内部边界

napcat/runtime/run_job的公开入口在mod.rs，能力类型和不变量在同一入口可见。journal处理签名记录，launch处理一次创建与原根恢复，service处理API归属/发现/恢复，prelogin处理登录前保留；内部文件不作为额外公开命名空间。

runtime_store的准备/复核仍保持同一状态事务中的显式顺序；输入信任检查、配置准备、持久意图和原生创建分别位于startup、runtime_store、start_intent和dispatch/run_job。不能为缩短函数而把锁顺序、持久化先后或未知结果边界隐藏在通用容器里。

媒体字节封装属于transport/media；消息侧的文件发布与引用/账号守卫属于messaging/media。两层共享预算来源，底层不导入消息用例。Job内核只处理原生句柄、成员和连接归属，上层保留事务不下沉为原生策略。
受检HTTP在transport/owned_wire内私有共享hyper实现与连接归属、预算和响应后守卫；transport/owned_http包装OneBot协议，napcat/control/owned包装固定WebUI路由。控制鉴权不使用账号API token，控制状态不产生账号身份或登记ready。协议模块和传输模块不互相反向依赖，运行恢复仍负责提供签名来源的完整身份与固定端点。

## 修改与验证

新增能力先选定所属职责；跨子系统编排放app，数据/协议规则放对应子系统，不在lib.rs添加旧根路径别名。单元测试放所属模块tests文件，集成/CLI测试在tests目录；测试中的高层组合依赖不扩散到生产实现。

scripts/check-architecture.ps1检查根入口、允许的子系统、应用入口审查预算、底层到应用/CLI的反向引用、传输到消息用例的引用以及unsafe允许位置。它是结构守卫，不能替代Rust类型检查、人工职责审查或行为测试。

scripts/check.ps1把结构/契约检查、发布包与版本锁合成检查、fmt/clippy、完整离线测试、release构建与源码包白名单串联。目录迁移还须核对include资源相对路径和独立进程测试的精确过滤路径，不能只看单个编译目标通过。

命令解析、帮助和输入Schema使用Clap定义与共享语义规则；Skill为手写Markdown的编译内嵌产物，命令路由有自动覆盖检查。RFC-0006/TASK-004记录已完成的单源契约整改，不恢复手写输出Schema或仅供Schema使用的结构副本。

实施状态（CHECK-058）：普通JSON和NDJSON均由实际Serialize + JsonSchema类型生成，无通用响应、LegacyWire、success(Value)或手写输出Schema。Clap实际类型提供输入字段约束，默认/必需/标志/冲突直接取Clap，条件依赖和组合预算由实际校验与Schema投影共享规则。Schema文档和原始协议、私有规范化指纹是受控JSON边界；不得新增任意JSON公共领域字段。运行权限、文件/签名、在线身份、UTF-8字节及时间先后仍需实际验证。

自动化见SPEC-005/RFC-0007：可复用Windows质量workflow执行同一本地入口，Semifold负责CD。scripts/package-release.ps1只组合Cargo元数据/固定目标构建/版本确认，release.psm1封装可独立验收的显式文件集打包与校验；check-release.ps1使用临时合成PE、ZIP及独立workspace验证保护和锁同步。内部架构和完整计划只在.agents，不进入源码包或发布ZIP。
