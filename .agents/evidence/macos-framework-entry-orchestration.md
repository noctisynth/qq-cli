# macOS Framework 入口编排合成验证

文档编号：CHECK-085。状态：Synthetic entry orchestration verified。日期：2026-10-24。
关联：SPEC-008、TASK-006。起点：0e27b6c。

entry.cjs由Rust资源常量提供给后续装配器，必须显式调用；本阶段没有部署、创建进程或修改QQ。使用原创惰性文件、独立临时目录与合成Electron/nativeLoader宿主，覆盖真实文件系统路径检查和模块调用次序。

8项入口测试覆盖：11个Electron路径先于Framework加载并回读，官方入口取得同一原生session后释放真实通知；错误架构/可执行文件/资源路径、晚启动和缺失目录拒绝；符号链接、硬链接、根重叠与父级跳转拒绝；Electron路径回读不符阻止加载；原入口异常保持且恢复dlopen；Framework同步/异步异常边界及固定错误码。补充初始化失败后迟到注册不能释放就绪。连同14项桥接测试共22项Node测试通过。

路径检查是形状与当前观察，不证明来源、签名、内容摘要、祖先ACL或检查后防替换。未接合生产入口生成、启动环境、专用副本部署与sandbox、失败持久状态；原型中launcher.installPathPkgJson兼容处理尚需独立版本审计。没有自动置前、截图、账号诊断或上游文件修改。

macOS门禁的fmt、Clippy、248项Rust测试及release构建通过（2项Rust测试忽略）；最终资源补充回调保护后重跑22项Node测试通过，release构建包含该资源。源码包269项白名单及全部src覆盖、git diff --check、Semifold status通过。阶段变更集macos-framework-entry-orchestration。未执行QQ或Keychain实验。
