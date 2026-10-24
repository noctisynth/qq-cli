# macOS 原磁盘 FD 释放内核

文档编号：CHECK-125。日期：2026-10-24。关联：FACT-005、SPEC-008、TASK-006。起点：ee8568d。

固定Apple源码身份与边界见FACT-005：DiskArbitration客户端对象仍以BSD字符串派发，不能仅凭CreateFromIOMedia命名宣称原对象效果绑定。IOStorageFamily普通打开FD保留minor槽位，DKIOCEJECT具独占/busy门槛；SDK27.0的sys/disk.h提供请求常量，原生代码不硬编码ioctl数值。无证据声称公开源码逐字等同当前安装内核。

初始原创合成HFS+镜像实验确认整盘/卷raw字符设备由当前用户拥有、0440且可只读打开；在挂载及其他open存在时SDK DKIOCEJECT返回EBUSY。保持raw FD后diskutil普通unmount成功，关闭卷FD后通过原整盘FD eject成功，hdiutil info中镜像条目消失，正常清理全部夹具。没有force、format或真实磁盘操作。

native/macos_disk是新增窄unsafe边界：Rust严格解析/dev/disk数字及分区路径，RDONLY/NOFOLLOW/NONBLOCK/CLOEXEC打开对应raw节点；原创C适配用fstat核对字符设备、当前UID及SDK主/次设备号。eject_once消费原File，guard/截止及身份检查后一次DKIOCEJECT，EBUSY/错误不重试、不换路径、不降级为force；成功只表示接受请求。构建只在macOS arm64编入适配，未新增依赖。

mounted/media读取并严格要求选定IOMedia的BSD Major/Minor，原Generation序列化不变；分区与整盘同设备号拒绝。Observation在原挂载根仍持有时打开两个raw FD，再重复验证系统映射、媒体代次、设备号及原目录，实现原文件系统读取者与原设备FD重叠。Devices重核原媒体代次和FD，提供固定卷名供后续普通卸载；eject前释放卷FD、消费整盘FD。此能力不序列化，不因观察获得挂载所有权；持久释放意图、独立监管、未知恢复和目录清理另行实现。

6项挂载定向测试通过，含缺失主设备号、相同主/次设备号拒绝；新增真实原创合成挂载测试覆盖错误minor拒绝、busy及guard拒绝后原挂载仍可复核、原FD跨普通unmount保留、一次eject及独立挂载映射消失。DMG字节不变，正常释放并清理夹具。现有同BSD名称重附加拒绝旧媒体观察仍通过。

完整macOS门禁通过：401项Rust通过、2项忽略，22项Node、fmt、Clippy、独立合成装配Complete/Unknown及arm64 release通过。随后在最终ioctl前补一次截止检查，6项定向挂载测试和Clippy再次通过；release含该检查。源码包354项包含原生C/Rust与新安装模块并排除内部/私人材料；受影响上下文链接、私人路径及diff检查通过。Semifold变更集macos-device-pins及status通过，共55项待发布变更集；仅本地提交，不推送或发布。

独立优化release宿主eject模式在固定官网DMG临时私有缓存上通过：生产受监管attach/m2完成、固定QQ Source完整认证及同请求恢复；在原挂载读取者仍持有时固定整盘/卷raw FD，然后释放Source、Observation及全部Ready，普通diskutil unmount成功，原FD重核及一次SDK eject成功。外部测试驱动独立确认hdiutil info中私有镜像条目消失、挂载点不再含QQ.app，然后移除临时缓存/宿主/状态。原始DMG的SHA-256、dev/inode/长度/mtime/ctime/模式/UID及可枚举扩展属性保持不变；未执行QQ或真实凭据操作。测试驱动释放不等于生产持久释放协议已完成。
