# macOS 磁盘附加身份

文档编号：CHECK-122。日期：2026-10-24。关联：SPEC-008、TASK-006。起点：0d2a3c0。

macOS SDK 27.0的IOKitLib.h中IORegistryEntryGetRegistryEntryID契约说明标识可跨任务识别注册表对象、仅在重启前有效；不能把孤立BSD设备编号当作附加代次。本机ioreg -a -r -c IOMedia报告含IORegistryEntryID及BSD Name，现有plutil JSON转换成功；初次受限沙箱查询失败，获授权的只读系统查询成功，未设置降级到磁盘编号的路径。
image/mounted/media复用有界系统查询和原生boot_session，绑定启动会话、整盘及卷的注册表对象ID。严格选择唯一、只读IOMedia及Whole标记，要求卷位于所选整盘祖先链内；预算为4MiB输出、4096节点、显式深度上限64并保留JSON解析器递归限制。Observation在初始返回及每次复核时同时检查系统映射、附加对象、固定Image、原目录和fstatfs；无新增unsafe、依赖或系统写入API。

5项定向测试通过。新增报告测试覆盖启动会话差异、零ID、错误类型、重复、断开祖先关系、错误Whole/可写标记、整盘/卷ID冲突、缺失启动会话及预算；同BSD名称不同对象ID产生不同观察。真实合成HFS+镜像测试正常卸载并再次附加同一文件到同一路径，本次实际重用BSD设备名；新对象ID不同，给新观察置入旧附加身份后复核拒绝。两个挂载均释放自有句柄后正常卸载，测试目录已清理，没有force或QQ操作。

首次完整门禁在既有媒体TLS正例失败（MEDIA_TRANSPORT），独立复测通过。夹具的服务器2秒/客户端3秒预算不适合作为冷系统TLS初始化的正确性门槛，单独提交440d0ec统一为10秒合成预算；生产策略和100毫秒超时反例不变。按门禁使用规范TMPDIR后4项TLS套件及fmt通过。最终完整macOS门禁通过：22项Node、fmt、Clippy、388项Rust通过/2项忽略、独立合成装配Complete/Unknown及arm64 release。源码包344项包含新模块并排除内部/私人材料；相关上下文链接、私人路径及diff检查通过。Semifold变更集macos-media-generation及status通过（含独立TLS修正共52项待发布变更集），仅本地提交，不推送或发布。

独立release宿主transition模式在固定官网镜像临时副本上通过挂载前快照/效果前核验、只读attach后的元数据转换、含启动会话/IOMedia身份的Observation以及固定QQ Source完整认证和重复核验。宿主退出后重新核对唯一镜像/设备，正常detach并清理临时副本/宿主。原镜像SHA-256、dev/inode/长度/mtime/ctime/模式/UID及可枚举扩展属性输出保持不变，副本挂载前quarantine与原值一致；未执行QQ或读写凭据条目。

这是磁盘附加对象身份，不独自证明文件系统从未重新mount，不授予attach/detach权威；一次挂载意图、监管及持久恢复仍待完成。Windows原生门禁未运行。
