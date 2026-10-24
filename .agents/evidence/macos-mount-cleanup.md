# macOS 挂载目录清理与后续请求

文档编号：CHECK-128。日期：2026-10-24。关联：SPEC-008、TASK-006。起点：3fe6aea。

m2版本2在效果前记录原空mount的device/inode，active目录同时绑定输入及请求。版本1仍按旧目录恢复，不补造原mount身份、不自动清理。Cleanup仅从已完成Release进入，先保存意图，再以描述符相对rmdir删除空mount和空父目录；阶段分别同步目录后持久记录。原请求及签名索引保留，只有Cleanup Complete放行同镜像新请求。

定向合成测试通过：释放前拒绝清理；额外内容、替换目录、符号链接和版本1记录均保留；五个持久/删除边界中断后可恢复。清理后实际新挂载采用不同目录，旧attach/release/cleanup请求保持历史结果，新挂载仍可读；随后新挂载正常释放并清理。独立进程分别在mount及parent的rmdir后、保存进度前被终止，争用请求返回busy，原记录恢复并完成清理。未使用force或递归生产删除，不启动QQ或访问真实凭据。

完整macOS门禁通过：410项Rust通过、2项显式Keychain测试忽略，22项Node、fmt、Clippy、独立合成装配Complete/Unknown与arm64 release构建通过。

固定官网DMG优化宿主组合通过：生产attach/Source/恢复/释放后进行目录清理，以新请求再次生产挂载；旧attach/release/cleanup保持历史结果，当前Source仍可重核。新挂载正常释放并清理，active目录为空，原请求索引保留。外部驱动再次确认镜像映射消失，原DMG字节、dev/inode/大小/mtime/ctime/模式/UID和扩展属性不变，临时场景正常清理，未执行QQ。

源码包363项分发边界、受影响上下文链接/本地路径和diff检查通过，已核对TODO-001/TASK-001专题导航与交付门槛。尚未清理Pending/Unknown attach、未完成释放、旧版无目录身份记录、镜像获取partial或应用部署失败现场；本阶段不等同完整生命周期交付。
