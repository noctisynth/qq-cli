# macOS 持久挂载释放

文档编号：CHECK-126。日期：2026-10-24。关联：SPEC-008、TASK-006。起点：fb389e5。

image/journal/release仅接受原m2认证Complete及固定私有缓存，比较完成指纹、原父目录/索引/缓存身份和原状态锁。原qq-mount.lock序列化释放。持有挂载根时认证原回执并建立raw FD，在保存绑定释放请求的Release(complete=false)之后释放本次根读取者，才向回调交付原FD。Ready.into_image提供消费本次根读取者的入口，调用者仍须释放其他Source/Ready。

回调返回码不是完成依据：要求原Image完整指纹不变，hdiutil中镜像/挂载点映射消失，原启动会话的整盘及卷IORegistryEntryID均不存在；IOMedia观察保留输出/深度/节点预算。重启后的不同boot意味着旧对象不再存在，但仍须镜像/挂载点映射不存在。全部核验后写Release(complete=true)。相同释放请求不重放，不同释放请求拒绝；panic/失锁保留未完成意图，恢复只读核验消失并补完成。原挂载请求得到Released而不重新attach。已完成释放返回历史状态，不授予新的读取/效果权限。

Complete挂载若在释放请求前即已消失，验证原镜像指纹和双重消失证据后先保存释放意图，再只读记完成，不构造或调用效果回调；没有模拟系统重启实验，原媒体同boot消失路径由实际合成卸载覆盖。原记录与active目录仍保留，其他挂载请求继续拒绝；生产释放效果的独立监管、目录清理、新请求接续仍须接合。

最终4项释放测试（含独立夹具入口）通过。实际原创HFS+挂载覆盖成功、空操作、报错、效果前panic、效果后panic；空操作不完成，所有重复请求不调用回调，不同释放请求拒绝。未释放的测试挂载由已知夹具正常detach后只读恢复完成。独立进程持有释放锁时另一请求拒绝，强制结束效果前/后的原创进程后分别恢复Unknown/Released，不重复效果。新增请求前已消失的完成挂载只读记录Released。原挂载请求不会重新attach。所有夹具正常释放/清理，无force、QQ或真实凭据操作。

完整macOS门禁通过：404项Rust通过、2项忽略，22项Node、fmt、Clippy、独立合成装配Complete/Unknown及arm64 release通过。门禁运行期间补入“请求前已消失”分支，随后最终4项释放测试串行全部通过；该新增用例不计入上述404。最终release构建已包含新分支。

最终Clippy、源码包357项的分发边界、受影响上下文链接/私人路径及diff检查通过。Semifold变更集macos-release-journal及status通过，共56项待发布变更集；仅本地提交，不推送或发布。

独立优化release宿主release模式在固定官网DMG临时私有缓存上通过：生产受监管attach、固定Source认证及恢复后，释放全部Source/Ready；生产m2释放日志调用测试驱动的普通diskutil unmount与原FD eject，独立双重消失观察后保存Released。重复相同释放请求返回Released且回调未运行，原挂载请求同样返回Released而不再attach。外部驱动再次确认镜像条目消失、挂载点不再含QQ.app，并清理临时状态/缓存/宿主。原始DMG的SHA-256、dev/inode/长度/mtime/ctime/模式/UID及扩展属性保持不变，未执行QQ或真实凭据操作。这证明生产日志与已知效果的组合，不声称测试驱动等于生产释放监管器或持久目录清理。
