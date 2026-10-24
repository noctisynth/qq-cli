# macOS 首次凭据与配置准备

文档编号：CHECK-117。日期：2026-10-24。关联：SPEC-008、TASK-006、CHECK-116。起点：be27aca。

按固定Framework提交859eb530c5801577633c2c3ae871d23586edf281的NapCatPathWrapper核对NAPCAT_WORKDIR：默认配置位于napcat-data/config而非XDG_CONFIG_HOME对应的实例根config；源码链接见FACT-005。本次仅固定源码观察，不宣称发布包已运行验证。l2布局版本提升2，纳入NapCat的config/logs/cache/plugins；旧凭据拒绝而不隐式改树。
macos_config在认证目录上无覆盖发布三个固定默认文件，检查0600/UID/ACL/flags/单链接、原目录身份、文件字节与inode；文件F_FULLFSYNC及目录fsync，失败保留现场。Published仅含目录指纹、文件身份和摘要，独立反序列化不能认证。
macos_material以r2 Pending绑定完整实例/安装/Plan、l2指纹和随机凭据包摘要；保留原生命周期锁，短状态事务复核原状态锁/快照及原记录。凭据读写和配置核验不持有状态事务。两个凭据键预检不存在，写后读回比对；记录不含明文token，配置文件按上游协议含必要token。全部通过后保存Complete，再提升prepared；已prepared重复请求不重写注册表。只读恢复不提升状态，缺失/变化不修复；Pending/Unknown不接续效果，不同请求拒绝。

4项配置测试覆盖配置目的路径、无明文凭据的观察序列化、账号配置保留、字节/权限/硬链接/同字节替换及guard/目录替换。7项准备相关测试覆盖原材料恢复、完成记录与状态提交间恢复、缺失/变化凭据或配置拒绝、写入失败/panic不重放、外来键/错误作用域、写入时状态/锁变化保留Pending、配置冲突转Unknown，以及丢失签名键时索引阻止替代权威。均为内存凭据，不访问真实Keychain。

完整macOS门禁通过：22项Node、fmt、Clippy、365项Rust通过/2项忽略、独立合成装配Complete/Unknown及arm64 release。后续对重复prepared请求避免注册表重写的修正及新增配置冲突场景，重新运行7项准备测试和fmt/Clippy均通过；合并去重共366项Rust通过。源码包330项包含新模块，排除内部/私人材料；受影响文档链接、私人路径和git diff --check通过。Semifold变更集macos-runtime-material及status通过，仅本地提交、不推送发布。正式CLI、运行后管理材料恢复、Keychain升级身份、真实QQ部署/启动及失败现场清理仍未验收；Credentials::set不是CAS，生命周期锁只序列化协作客户端，不宣称恶意同用户隔离或断电验证。
