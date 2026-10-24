# macOS 释放效果监管

文档编号：CHECK-127。日期：2026-10-24。关联：SPEC-008、TASK-006。起点：e7f385f。

image/release在m2首次释放意图回调内消费原Devices，传递原整盘/卷raw FD及有界媒体元数据。worker核对继承FD的只读字符设备、当前UID/主次设备号及boot/IOMedia代次；不按设备名重新打开。元数据本身不授予释放权。原FD复制为CLOEXEC后，将stdout/stderr替换为null，避免额外卷FD阻止eject或泄露给普通卸载命令。

attach/release共享supervision许可与存活协议。父端复核原意图guard才发送字节1，EOF/额外输入/截止取消；直接命令由worker有界回收，父端不杀监管者。worker固定diskutil普通unmount，再消费原FD执行一次SDK DKIOCEJECT；不force、不重试。内核未及时返回保留未知，不声称回收磁盘服务；完成仍由m2镜像指纹和双重消失核验确认。

实际CLI合成集成4项通过：已有attach原描述符/许可测试，以及释放错误普通文件FD、卷FD冒充整盘、错误媒体ID/设备号/boot、缺失卷声明、EOF/额外许可拒绝；每次拒绝后原合成挂载仍在。成功路径实际卸载/eject后独立查询镜像/媒体消失，合成镜像字节未变，正常清理夹具。畸形私有入口静默拒绝。未启动QQ或访问真实凭据。

完整macOS门禁通过：407项Rust通过、2项显式Keychain测试忽略，22项Node、fmt、Clippy、独立合成装配Complete/Unknown恢复及arm64 release构建通过。共享监管测试覆盖EOF/guard/截止、父进程突然退出后直接命令回收及未知退出，不由此承诺内核阻塞可回收。

优化release独立宿主在固定官网DMG临时私有缓存上通过生产attach→Source认证→只读恢复→生产release；释放调用已使用本阶段真实独立worker，不再由探针提供diskutil回调。m2独立确认消失后返回Released，重复释放及旧挂载请求保持Released。外部驱动再次确认镜像映射消失、挂载点无QQ.app；原始DMG的SHA-256、dev/inode/长度/mtime/ctime/模式/UID及扩展属性未变，临时场景正常清理，未执行QQ。固定资产基线沿用CHECK-119/126。

源码包361项，未包含内部上下文或私人研究材料；受影响上下文链接、私人路径及diff检查通过。已核对TODO-001/TASK-001的专题导航与完整交付门槛，无需重复更新。本阶段不完成目录清理、新请求接续及正式CLI启动。
