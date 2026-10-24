# macOS 实例部署日志

文档编号：CHECK-110。日期：2026-10-24。状态：并发、中断、完成恢复及完整macOS门禁通过。
关联：SPEC-008、TASK-006。起点：ebec7e0。

新增runtime/macos_deploy：以runtime_store既有(instance_id,run_id)摘要生成独立索引及固定QQ.app路径，d2认证记录绑定Target与request_id。注册表实例/安装唯一匹配、prepared且无账号/进程、目标不存在才发布Pending并进入一次操作。原生命周期锁跨操作保持，状态检查用短事务；记录最初状态锁目录/文件身份，拒绝换锁后继续。guard复核完整注册表快照和原Pending；失败/中断不再次派发，不同键不能占用同一代次绕过旧记录。

Inventory增加私有签名完成标记，仅全部签名、权益、整树核验及同步成功后设置。Artifact消费活跃副本并复核后记录整树摘要、app/父目录设备与inode及UID；普通扫描不能生成。Complete恢复验证原认证记录、当前作用域/输入、固定路径、父目录私有权限/ACL、原app身份/全部UID及完整指纹，不返回写入能力或启动权限。d2新增到受限kind集合，macos-deployments索引阻止原签名密钥丢失后再造权威。

合成测试：操作错误、Rust展开中断均仅执行一次，原键返回Unknown，新键冲突；普通扫描生成的unsigned树不能登记Complete；操作中注册表变化或状态锁文件替换使guard失效，保留未知结果。两个独立进程竞争同一意图，效果文件仅写一次；独立进程在产生效果后直接exit，Pending保持，跨进程原键/新键均不能重放。

完成正例用原创测试程序app，执行私有复制、加固、完整签名/同步、消费Artifact、发布Complete；恢复及再次调用部署均验证原副本而不执行回调。不同输入摘要、错误签名密钥及资源修改均拒绝；原始来源保全。密钥遗失测试使用内存凭据，只有部署索引存在时仍不写入新密钥。未读写真实Keychain、QQ或账号。

此内核的input_hash由调用方对已认证固定输入产生；格式检查本身不证明来源/版本/适配入口正确。真实装配生产者、失败清理及恢复写权限、CLI启动和断电实验未完成。长复制/签名阶段不占用状态事务；只读完成恢复仍需当前状态锁和完整树读取。Windows原生/PowerShell门禁未运行。仅本地提交，不推送/发布。

最终门禁通过：22项Node、fmt、Clippy、340项Rust通过（2项忽略）、arm64 release构建。源码包313项全部源码覆盖且排除内部/私人材料，证据唯一编号/相对链接、git diff --check及Semifold status通过。阶段变更集macos-deployment-journal。
