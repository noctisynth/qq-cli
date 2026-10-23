# macOS 原创 Framework 适配核心

文档编号：CHECK-084。状态：Synthetic adapter core verified。日期：2026-10-24。
关联：SPEC-008、TASK-006。起点：219776d。

将 CHECK-072 的原创适配逻辑提取为维护中的 bridge.cjs，以 Rust include_str 暴露源码；移除实验诊断、账号与私有配置读取，不复制或修改上游。模块加载无自动安装效果，尚未接合生产入口、路径审计或专用副本部署。

14项Node合成测试覆盖：注册/通知先后、仅接受真实布尔成功、过期代次、一次释放、不可写原生方法与冻结监听器转发、原回调及init异常、不读取私有配置、仅官方nt_1 getter转交同一原生session、缺失数据路径逐次回退、绝对路径检查、dlopen receiver/参数/返回/异常、冻结导出拒绝、恢复不覆盖后续观察器。

scripts/check-macos.sh通过14项Node测试、fmt、Clippy、248项Rust测试及arm64 release构建；2项Rust测试忽略不计入通过。Node版本22.20.0，无npm依赖。源码包267项，包含适配核心与测试；未执行真实QQ操作或待授权Keychain查询。此结论不覆盖真实版本兼容、生产启动、切号/重连或完整CLI交付。
