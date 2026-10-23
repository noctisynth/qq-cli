# macOS 启动准备与入口生成

文档编号：CHECK-086。状态：Synthetic preparation verified。日期：2026-10-24。
关联：SPEC-008、TASK-006。起点：997f45e。

新增macos_adapter::Prepared，读取既有目录并生成专用副本程序路径、cwd、唯一user-data-dir参数、固定8项环境及main.cjs源码。不复制、部署、签名或创建QQ进程，不读取Keychain。路径形状检查不证明文件来源、权限、版本或未来操作时身份不变。

3项Rust定向测试通过：环境仅包含固定集合、系统PATH与实例数据路径，参数为单字面量，检查不创建QQ.app；拒绝重叠/缺失目录及符号链接；实际生成的main.cjs在Node隔离VM中执行，固定require替身检查路径/host传递、正常/同步失败/失败回调，确认带引号、反斜杠、中文及JS样式文本的目录只作为数据。无Electron或QQ代码执行。

准备环境与原型继承环境不同：HOME/CFFIXED_USER_HOME及固定LANG需后续真实兼容验证。生成入口的exitCode仅标记本进程错误，不意味着QQ树已退出；生产编排须持久记录并收敛失败。原锁复核、文件原子发布、专用副本审计、sandbox、受控创建与签名记录生产者仍未完成。

完整macOS门禁通过：22项Node测试、fmt、Clippy、251项Rust测试（2项忽略）及arm64 release构建。源码包271项白名单与全部src覆盖、git diff --check及Semifold status通过。阶段变更集macos-launch-preparation。
