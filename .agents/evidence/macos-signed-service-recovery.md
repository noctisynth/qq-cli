# macOS 签名服务记录的只读恢复

文档编号：CHECK-083。状态：Synthetic application recovery verified。日期：2026-10-24。
关联：SPEC-008、TASK-006。起点：dc1388b。

## 接合范围

新macos_run只读取独立macos-runs/j2记录，校验签名、macos平台、唯一注册表快照、安装/运行/端点及原s1 Dispatched回执，才恢复原进程。应用backend和事件SocketSource接入此前原socket身份守卫；不会回退裸HTTP或裸WS。
生产代码没有写入j2/任意PID登记入口；本次记录由测试用固定合成签名密钥构造，服务为原创测试进程。只读恢复不能替代受控启动生产者、服务首次发现或完整运行所有权。

## 验证

合成应用测试覆盖：从签名启动回执与j2恢复到受检HTTP，确认预期路由/Header认证及OneBot响应；错误版本/平台、旧j1引用、HTTP/WS端点变化、原输入摘要不符、原身份变化、篡改字节、重复安装记录拒绝；Pending/Unknown原启动回执拒绝。缺失记录仍先于凭据访问和状态创建。
补充应用SocketSource的真实回环WS握手/Header认证测试。原native socket测试继续验证错误owner/监听者无认证字节，不能仅以本次正例代替负例。
补充macos-runs索引存在但引用文件缺失时不创建替代签名密钥的测试。新增服务恢复测试使用内存签名凭据，不访问系统Keychain；未执行真实QQ操作或待授权的Keychain元数据查询/清理。
scripts/check-macos.sh的fmt、Clippy、248项普通测试及arm64 release构建通过。源码包264项白名单及完整源码覆盖、git diff --check和Semifold status通过；阶段变更集macos-signed-service-recovery。尚无生产受控启动/记录写入、控制面恢复、完整CLI真机读取、停止恢复或Windows原生回归结论。
