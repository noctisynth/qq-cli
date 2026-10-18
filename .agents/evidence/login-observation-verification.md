# 登录观察实现验收

文档编号：CHECK-019。状态：协议/受检HTTP/只读恢复及公开status离线通过，真实原版待验收。更新：2026-10-19。
关联：CHECK-016、SPEC-004、RFC-0003、TASK-002。

当前有效验收：公开待登录status含3项独立CLI和1项身份分类/版本先行单元测试，完整231项离线回归及fmt/clippy/release/架构/174文件源码包通过。真实原版及完整登录恢复未验收。下文按实施顺序保留早期阶段的数量和“尚未接入”记录，不覆盖本段当前状态。

napcat/control实现固定POST路由、SHA-256(token+.napcat)管理鉴权及CheckLoginStatus；Backend要求在发任何凭据前证明同socket归属。观察最多两次调用共用截止，2FA/拒绝/异常立即结束，不触发QQ登录、刷新、密码、快速登录或设备接口。64KiB响应预算；Credential私有且不实现Debug/Serialize，不持久。状态只输出固定八种阶段（含unknown）及四个布尔提示，无账号/二维码URL/错误正文；阶段不决定账号ready，不一致布尔拒绝。

4项原创纯协议测试覆盖固定独立hash向量、七阶段及未知字段脱敏、初始化阶段与在线提示独立、2FA/拒绝/坏凭据零后续请求、严格字段/类型/重复code、冲突布尔、异常JSON/字节预算/传输失败及过期截止不重试。cargo test --offline --locked -j1 --lib napcat::control全部通过，全目标clippy -D warnings通过；fmt/架构/源码包检查见补记。此次没有新增网络实现或真实QQ操作，不代表公开status已提供控制阶段或原版登录恢复完成。此前完整220项证据仍见CHECK-018，新模块增加4项，尚不称完整224项回归已通过。

提交前fmt、架构检查和167文件源码包检查通过，Semifold status识别34个changeset、仍计划0.1.0到0.2.0，不消费或发布。包许可元数据既有警告保留。

## 受检HTTP增量

共享transport/owned_wire私有hyper POST及原socket的Job/Pinned验证，OneBot包装接口保留。napcat/control/owned实现固定路由和请求形状检查，鉴权只传hash、不带Bearer，状态只传服务器Credential和空body；每次连接独立认证归属，64KiB上限与更低配置预算生效。无重定向、重试、代理或query凭据，不改变身份/登记状态。

8项owned_transports测试通过，包含新增2项管理测试：IPv4/IPv6正常鉴权及脱敏观察、2FA/拒绝/重定向只发一次、超大状态拒绝；错Job/端点零应用字节，过期截止及非法请求形状零请求。原OneBot固定/分块/媒体/错误预算和WS及签名CLI测试保持通过。最终完整226项离线回归、全目标clippy、fmt、release、架构和169文件源码包检查通过，编号/隐私及Git diff检查通过。Semifold36个changeset，计划版本不变，不消费或发布。缓存硬链接回退及包许可元数据既有警告保留。未调用真实QQ/NapCat，控制端点仍由测试给定，公开CLI恢复未接。

## 原控制能力及秘密只读恢复

run_job/control仅按当前完整实例/安装快照重开签名j1的原prelogin进程/Job，核对映像/固定端点及已有API同进程/端点；必须有原t1保留凭据，不连接、不扫描、不再次retain、不修补旧日志。runtime_store/recovery独立于verify_prepared，只读原r1/最终marker和原vault，核对计划、端点、版本及秘密引用，不要求live日志/缓存为空，不读取其正文。返回管理材料不实现Debug/Serialize，不创建状态、秘密或目录。两能力尚未在应用层组合，因此不把秘密存在当作进程权威。

11项runtime_store定向测试通过：新增1项覆盖运行后日志/缓存容忍、原管理token区别API token、换run/marker/安装拒绝及缺秘密不补建；扩展原父子预登录测试核对只读控制重开与丢t1拒绝，保持无新增保留日志。最终完整227项离线回归、全目标clippy、fmt、release、架构及171文件源码包通过。Semifold37个changeset，计划版本不变，不消费或发布；既有警告保留。公开status、真实原版及完整登录恢复均未验收。

## 公开待登录status

app/status/pending组合原签名控制能力、原管理秘密和受检管理HTTP，核对两端点相等；网络期间不持状态锁，返回前重读完整实例/安装快照。未绑定旧记录保持未验证状态，已有错误不降级。starting/login_required的账号服务未绑定/监听、或受检OneBot明确缺身份/连接拒绝可观察控制；版本、协议、鉴权和归属错误不掩盖，ready错误不降级。输出loginObservation，account:null、identityVerified:false、无meta.scope；管理ready也不写账号或登记ready。schema/status严格区分未验证提示与OneBot账号作用域，公开文档和runtime skill同步。

3项新增独立CLI/同Job原服务定向测试通过：控制阶段及ready不确认账号、API初始化仍无身份提示、版本拒绝及OneBot真正就绪后才记实际账号；2FA/拒绝只发一次，状态响应中另进程持锁改run证明网络不持锁且输出前拒绝；缺t1/r1/vault不修补也不向后端发管理鉴权。每条实际CLI成功/失败输出由独立JSON Schema验证，正文/挑战URL/凭据/私人目录不输出。服务仅原创测试代码，不运行QQ或NapCat。

初轮夹具失败：Windows接受socket继承非阻塞模式造成提前关闭，修正为阻塞并保持生产归属守卫；失败构造残留子程序锁住测试exe，建立先行RAII清理并持只读进程句柄有界等退出，CLI子进程也有测试总截止。计数在响应前发布以避免断言竞态。上述是测试夹具修正，不构成上游兼容证据。完整回归及工程检查完成后补记。

最终完整230项离线回归、fmt、全目标clippy -D warnings、release、架构及174文件源码包通过。Semifold39个changeset，计划仍0.1.0到0.2.0，不消费或发布；既有缓存硬链接回退和包许可元数据警告保留。任务把已验收的待登录status独立拆为已完成项，真实原版及完整登录恢复未勾选；业务60/93、运行环境20/29，未完成仍33项，不代表工作量百分比。

提交前复核补充边界：status在读身份前使用底层版本守卫，不增加版本请求；版本不匹配且身份也缺失时仍拒绝，不进入控制面。原严格CLI两次版本拒绝各少读身份/在线2项，精确请求计数25→21，其他行为不变。连接拒绝单独记BACKEND_NOT_LISTENING（无请求发送），只有此类或缺身份可在待登录观察控制；含坏HTTP帧的一般BACKEND_TRANSPORT必须原样报错。独立CLI夹具扩展坏版本+无身份、坏HTTP不降级、已有API绑定但停止监听后仍能从原控制观察，完整最终复核完成后更新。

最终边界复核已完成：新增1项单元测试区分缺身份与坏对象/标识/类型，版本守卫拒绝时不读身份；3项CLI测试含坏身份不降级。最终完整231项测试、fmt、全目标clippy、release、架构、174文件源码包、编号/隐私与Git diff检查通过。上下文计数60/93与20/29已交叉核对，真实原版任务仍未勾选。Semifold39个changeset，不消费或发布；既有警告保留。

后续空状态补修：发现无实例status返回not_started未被新Schema覆盖，补入枚举并显式identityVerified:false。原无HOME/无实例/不创建状态的真实CLI测试增加独立Schema断言；10项cli与3项status_login共13项通过，全目标clippy通过，fmt/架构/174文件源码包通过。未新增测试数或业务勾选，最近完整231项及release仍为上述基线，本小修未重跑全量或release。Semifold新增patch:fix后40项，不消费或发布。
