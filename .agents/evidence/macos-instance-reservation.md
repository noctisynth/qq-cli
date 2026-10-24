# macOS 固定来源实例预约

文档编号：CHECK-118。日期：2026-10-24。关联：SPEC-008、TASK-006、CHECK-115/116/117。起点：2937be0。

macos_creation在原生命周期锁下复核固定官网Source、唯一v4.18.41安装/i1及原状态锁身份。长文件校验不持有状态事务，前后通过短事务检查安装快照。共享instance_creation::reserve_in登记协议，macOS签名类型n2，既有入口保持n1；同一请求索引遇到另一类型拒绝，不迁移旧权威。协议在发布意图、持久化注册表及返回前复核guard/原状态锁；额外拒绝重复实例ID或其他实例占用同一代次。
登记继续先发布Reserved，再持久化preparing实例，最后Registered。Fresh保留三个回环端口租约；中断恢复原ID/端口，不选择替代端口，已移除实例不重建。预约不创建QQ进程、配置、运行目录或凭据，也不把固定来源认证当作启动兼容性。

8项预约测试通过，含原有并发/独立进程、稳定请求、消息键冲突、中断恢复和端口争用；新增n1/n2相互拒绝及发布claim后丢失guard的原ID恢复。第一次定向调用因临时目录别名触发UNSAFE_PATH；按macOS门禁相同规则使用规范临时目录后全部通过，没有修改路径安全检查。

独立宿主source模式使用已校验SHA-256的固定官方DMG，只读挂载并认证原树，导入固定Framework ZIP到全新合成状态。通过实际macos_creation生成随机实例/代次和端口，接合l2目录、内存凭据r2配置及macos_assembly只读计划；改变注册表端点后旧计划拒绝。确认adapter、QQ.app和d2索引不存在；没有真实QQ部署、签署、执行、登录或OneBot操作，没有Keychain访问。成功后合成状态/宿主清理、只读DMG正常卸载。

首次完整门禁出现既有合成TLS测试MEDIA_TRANSPORT失败，该测试独立复测通过，未改TLS代码；最终独立重跑完整macOS门禁通过：22项Node、fmt、Clippy、368项Rust通过/2项忽略、独立合成装配Complete/Unknown及arm64 release。Semifold变更集macos-instance-reservation与status通过，仅本地提交、不推送发布。源码包331项包含新模块并排除内部/私人材料，当前diff与上下文链接/私人路径检查通过。正式CLI路由、来源获取/挂载监管、运行兼容性和隔离启动、生命周期恢复仍待接合；Windows原生回归未运行。
