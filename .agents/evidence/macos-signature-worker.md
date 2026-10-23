# macOS 签名工作进程隔离

文档编号：CHECK-091。状态：Synthetic lifecycle and installed-source verification passed。日期：2026-10-24。
关联：SPEC-008、TASK-006。起点：879c773。

inspect_original的签名步骤改为当前CLI私有只读工作进程，不继承环境且无shell；固定退出码、不输出私有错误。父端轮询截止并关闭存活管道，仍存活的原直接Child请求终止并有界回收。子端独立计时器和存活管道线程可结束阻塞于系统签名调用的进程，不需要该调用返回。无法回收明确报未知；不按持久PID记录清理QQ。

新增单元测试覆盖预期/未知退出码、父端截止、原进程退出观察、迟到成功拒绝，以及模拟阻塞主线程时独立计时器/父管道EOF使子进程退出。普通fixture测试自身无工作时直接返回。新增CLI集成测试覆盖私有参数校验、无公共输出和关闭父管道；已有ad-hoc负例还通过实际CLI工作模式验证，保持stdin打开直到原生拒绝完成，不用EOF冒充签名失败。所有普通测试仅原创临时文件/程序，不读取实际QQ。

临时宿主注册同一worker入口后，用新Rust组合接口只读验证已安装QQ成功：origin=AppleAppStore，version=7.0.2-53644，指纹5fa6d9836ad144f7ebbc62434182fdb54cf06cbaa6e49b045a9600d6db8fb350，耗时约32.70秒（120秒预算）。未启动/修改QQ或读取会话。CHECK-090的30秒直接调用失败仍保留为历史证据。

这里结束的是本工具的临时检查进程，不是QQ或系统信任服务。2秒清理宽限未确认退出会明确失败；不声称任意内核阻塞/调度条件下绝对时间保证。真实QQ部署、签名修改与整个运行树管理仍未由此完成。

完整macOS门禁通过22项Node测试、fmt、Clippy、271项Rust测试（2项忽略）及arm64 release构建。之后收紧查询错误时不得终止的清理分支，重跑5项工作进程单元测试、全目标Clippy和arm64 release构建通过；另有默认本机release构建通过。源码包281项白名单及全部src覆盖、git diff --check、Semifold status通过。阶段变更集macos-signature-worker。
