# macOS 只读挂载观察

文档编号：CHECK-120。日期：2026-10-24。关联：SPEC-008、TASK-006。起点：1458dc2。

image/mounted组合固定系统hdiutil info -plist及plutil JSON转换；清空环境、固定绝对工具路径/参数、不经shell、不接受外部程序选项。输入匿名文件和非阻塞stdout有4MiB上限，使用截止、guard与原Child退出确认；错误只终止并回收本次直接查询进程，无法确认退出时报告MOUNT_QUERY_EXIT_UNKNOWN，不从查询进程退出推断挂载清理。无新依赖或unsafe边界。
机器报告要求唯一原镜像路径、当前UID、非writeable/未加密、无重复设备且同一严格解析的disk根、唯一精确挂载点。再打开规范目录并通过fstatfs验证MNT_RDONLY、实际设备、实际挂载根；重复观察核对原目录和当前路径dev/inode以及系统报告。Observation借用固定Image并持有目录句柄，不可反序列化，不授予attach/detach或运行权限。

4项定向测试通过：机器报告中的权限/写入/加密/重复/设备/路径/缺失字段拒绝，普通目录伪装拒绝，系统查询非零退出/超时/guard/超量输出，以及本次原创HFS+合成镜像的只读挂载、内容读取、重复观察、子目录拒绝、正常卸载后新观察拒绝。测试只操作自己的临时镜像，成功正常卸载并清理。
前两次合成实验暴露实际接合要求：只读attach可能更新镜像校验缓存属性/ctime，因此不能沿用挂载前的严格Image观察；正式实现须单独认证该转换，本次观察使用挂载后全量认证Image，未放宽旧stamp规则。活跃目录句柄会让正常detach返回busy；测试释放自有句柄后正常卸载，不加force。两次失败现场均已核对并正常清理。镜像错误文本移除“未挂载”的不适用承诺。

最终完整macOS门禁通过：22项Node、fmt、Clippy、384项Rust通过/2项忽略、独立合成装配Complete/Unknown及arm64 release。源码包341项，包含4个新增模块且排除内部/私人材料；上下文链接/私人路径与diff检查通过。Semifold变更集macos-mount-observation及status通过（49项待发布变更集），仅本地提交，不推送或发布。

独立release宿主mounted模式通过固定官方镜像临时副本的Image、Observation与QQ Source完整认证及重复核验。原镜像保持SHA-256、dev/inode/长度/mtime/ctime/模式/UID及可枚举扩展属性输出不变；副本quarantine在挂载前与原值比较一致。宿主退出后重新核对唯一镜像/设备，再正常detach；临时副本和宿主已清理。没有执行QQ、读写会话或访问Keychain。首次宿主编译因缺少目标构建的host依赖搜索路径退出，未发生挂载；补齐搜索路径后完整通过，未修改产品代码。系统工具报告hdiutil attach/detach已弃用但本机仍可用，最低macOS版本及长期工具选择仍随分发验收核对。

生产attach/detach、一次意图/监管、持久恢复、上游原镜像属性转换及CLI接合仍待完成；不把读观察当作挂载权威或QQ验收。Windows原生门禁未运行。
