# macOS 启动效果边界的原锁重核

文档编号：CHECK-082。状态：Synthetic dispatch recovery verified。日期：2026-10-24。
关联：SPEC-008、TASK-006。起点：d512d46。

## 改变

CHECK-081提供原锁观察，本阶段将其接入通用start_intent::Intent::dispatch。效果回调前先重核；效果后、保存回执前后再次重核。效果可能发生后的失锁使用原START_OUTCOME_UNKNOWN/request_id并禁止安全重试，不刷新原锁或在已确认失锁后回写新位置。
macOS使用原文件/目录身份；Windows仍使用原持有句柄，不改变其既有操作与记录格式。尚未新增macOS运行日志格式或允许启动原型直接登记服务。

## 验证

新增合成测试覆盖回调前/后×锁文件/状态目录替换四种情况：效果前失锁回调零次；效果后回调一次并返回Unknown；替代目录保持为空；原目录恢复后仍只读取Existing/Unknown，不能再次派发。
start_intent八项测试通过，包含并发单次机会、独立进程/突然退出、丢回执及不可重放行为。scripts/check-macos.sh的fmt、Clippy、242项普通测试及arm64 release构建通过；最终回执后重核补充后，八项启动意图测试和all-target Clippy再次通过。源码包262项白名单/源码覆盖及git diff --check通过。Semifold变更集macos-dispatch-lock-guard；Windows原生回归未执行。
未操作QQ，未执行Keychain查询/清理；不证明所有运行操作均已接入该检查，也不证明完整macOS日志迁移、启动或停止交付。
