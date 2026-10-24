# macOS 镜像挂载元数据转换

文档编号：CHECK-121。日期：2026-10-24。关联：SPEC-008、TASK-006、CHECK-120。起点：a90ca5d。

MountTransition消费挂载前的严格Image与描述符保护/属性快照。完成时重新完整校验固定长度/SHA-256、同一dev/inode/mtime及原句柄，检查UID/GID/模式/链接数/flags/出生时间/ACL不变。复用安装层Attributes扫描，唯独允许4096字节以内的com.apple.diskimages.recentcksum新增或更新；既有该属性不得消失，其他属性（含quarantine、压缩及资源叉）不可新增/变化/删除。verify_before在效果前要求原Image及保护信息/全属性完全未变，拒绝准备后的变化；成功完成转换才返回新stamp的Image，旧Image严格重核不变；超时、失guard或不允许差异只拒绝，不写入修复。

新增3项转换测试：校验缓存新增/更新及无变化通过、后续变化仍被严格Image拒绝，quarantine/普通属性新增修改删除/缓存删除超限/模式/同字节替换/字节变化/硬链接/mtime/ACL/flags反例，以及截止与guard失败。现有真实合成HFS+挂载测试改为挂载前准备转换、挂载后完成转换，再接合Observation；句柄释放后正常卸载。扩展flags反例首次因测试使用错误的chflags绝对路径失败，已改为系统实际路径，未修改产品检查。

最终代码完整macOS门禁通过：22项Node、fmt、Clippy、387项Rust通过/2项忽略、独立合成装配Complete/Unknown及arm64 release。此前一次门禁也通过，随后增加verify_before及其反例，故对最终代码重新执行完整门禁；不以旧构建覆盖新增接口。源码包343项含两个新模块且排除内部/私人材料，上下文链接/私人路径及diff检查通过。Semifold变更集macos-image-mount-transition与status通过（50项待发布变更集），仅本地提交，不推送或发布。

独立release宿主transition模式先对固定官网DMG临时副本准备MountTransition并verify_before，报告PREPARED后测试驱动才执行只读attach；随后宿主finish转换并接合Observation、固定QQ Source完整认证及重复核验。宿主退出释放句柄后重新选择唯一镜像/设备并正常detach，临时副本和宿主已清理。原镜像的SHA-256、dev/inode/长度/mtime/ctime/模式/UID及可枚举扩展属性输出保持不变，副本挂载前quarantine与原值一致。测试没有执行QQ、读取会话或访问Keychain。

本转换不执行或证明attach、不授予卸载权威；一次挂载意图、执行监管、持久恢复及CLI接合仍待完成，Windows原生门禁未运行。
