# 受检直接JSON调用与持久回执

文档编号：CHECK-139。状态：离线JSON调用切片通过。日期：2026-10-29。
关联：RFC-0009、SPEC-009、TASK-007、CHECK-138。

api call支持动态ActionName、参数文件/stdin/空对象及离线dry-run，名称不由固定目录封锁；OneBot原生status/retcode/data/echo及未知字段在api专属动态JSON边界保真。业务拒绝为非零退出并保留显式结果，异步确认不是最终完成。
使用既有selection、平台认证运行与owned HTTP；仅在所选固定端点发送，不新增外部URL/token入口。受限targets仅对可核对目标的已知调用开放，读/发送/媒体策略继续生效。
写入及未知效应沿共享requests ledger，副作用前保存api.ACTION及带密钥参数摘要；不保存原始参数或响应。已有消息/启动请求键互斥继续生效，同账号跨实例复用，同键异参/异账号/异操作冲突；pending和unknown均不重放。request get离线返回api.call/action/outcome与resultAvailable:false。
参数JSON拒绝重复键/越界数值/超深与超大输入，开启精确数值及RawValue以避免CLI先行浮点舍入。动态字段仅位于core/contracts/api/json，契约门禁递归检查并具名排除此协议文件，不放宽领域DTO。

原创受管HTTP/独立CLI覆盖未知点号action、账号/版本确认、完整成功/失败响应、派生Schema、重复调用不联网、离线查询与异参冲突；使用临时签名权威和合成账号/正文。第一轮夹具遗漏现有server.start readiness步骤，修正后通过，不计失败为产品故障。
4项API单元测试覆盖参数键顺序、敏感不落盘、同键跨实例/账号/操作、网络未知/账号变化/异步/拒绝、pending中断及输出超限不重放、目标限制及禁止发送前零业务调用。离线CLI测试补重复键、精确数字、输入对象形状、未知缺键的前置拒绝与dry-run无配置访问。
完整串行cargo test离线回归通过；新增4项API用例最终单独复核通过。all-targets Clippy、Windows release构建、结构/单源契约、Skill验证、源码包白名单434文件、fmt/diff和发布CLI离线冒烟通过。
发布归档/版本锁合成检查通过。安装器PS7及PS5.1分别出现一次File.Replace异常，单独复跑均通过，未修改安装器、未认定根因已修复；记录为合成环境的偶发限制，不据此声明安装升级稳定性完成。

限制：未执行真实QQ操作或macOS直接API真机验证；全接口参数/效应核对、领域任务、流式协议与直接API专项跨进程/真实生命周期故障矩阵未因此完成。上下文中的178/170仍为固定范围基线，不是当前完整功能通过数；后续按TASK-007逐组实施。
