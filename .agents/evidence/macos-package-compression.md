# macOS 压缩入口部署

文档编号：CHECK-100。日期：2026-10-24。状态：合成压缩部署及macOS离线门禁通过。
关联：SPEC-008、TASK-006。起点：3143d29。

原创临时合成文件证明：以只读打开时UF_COMPRESSED及隐藏decmpfs仍存在；可写打开本身就可解压、清除压缩标记/属性并改变mtime，尚未调用write也会改变文件。不能把只在write之前复核当作覆盖全部效果边界。Python首轮探测因本机os模块没有setxattr失败，后改为系统xattr/chflags构造原创夹具；没有接触真实QQ。

deploy_package现在在可写打开前检查原guard、父目录、适配资源、整树stamp和只读package句柄身份。对压缩目标由内核正常打开处理解压，不调用手动清除属性/标志，也不新增unsafe。完整同步并重扫后，仅允许目标UF_COMPRESSED与decmpfs消失、系统删除压缩资源叉、mtime/ctime变化；目标逻辑内容、inode/属主/模式/ACL/链接数保持。仍存在的资源叉必须字节一致，quarantine和所有其他属性保持且不能增加；其他节点完全不变。核验后再次检查guard及stamp，才执行原有入口写入。失败消费能力并留现场，不能据已解压状态报告部署成功或自动重放。

新增原创zlib JSON夹具覆盖内联压缩、内联压缩加独立资源叉、资源叉压缩。验证原来源保全、最终main改写、压缩标记消失、独立资源叉保留、压缩资源叉被系统移除、quarantine和普通属性保持；覆盖可写打开前失权未解压、解压后失权不改入口、解压后quarantine/其他树节点改变拒绝。属性差异单元测试独立拒绝丢失quarantine、修改资源叉、新增属性、保留decmpfs和未压缩来源的伪规范化。

格式调查：Apple公开XNU固定提交f6217f891ac0bb64f3d375211650a4c1ff8ca1ea的[decmpfs.c](https://github.com/apple-oss-distributions/xnu/blob/f6217f891ac0bb64f3d375211650a4c1ff8ca1ea/bsd/kern/decmpfs.c)，检索2026-10-24，说明压缩文件隐藏decmpfs及资源叉，不能从默认可见属性推定全部元数据。参考[libarchive资源叉格式说明](https://github.com/libarchive/libarchive/blob/master/libarchive/archive_write_disk_posix.c)构造原创单块资源叉；此链接为可变来源，不作为固定复现基线，不复制上游实现。复现基线为仓库原创SOURCE/ZLIB及资源叉字段夹具；不宣称固定XNU源码就是本机操作系统版本。

无真实应用复制、签名或启动。本阶段规范化委托宿主支持的压缩解码器，合成实测为zlib内联/资源叉两种表示；不据此宣称所有压缩编码及所有macOS版本已验证。未知或无法解码的来源仍拒绝，不把接口能力当作支持组合结论。持久部署恢复、签名、启动、真实兼容及父端绝对预算仍未完成。

完整macOS门禁通过：22项Node、fmt、Clippy、304项Rust通过（2项忽略）及arm64 release构建。复查补充普通文件可写打开后再次检查截止/权限，两个部署集成测试再次通过，新增该边界失权情形后定向测试、最终fmt/Clippy通过；最终源码已用于本轮release构建。源码包295项、全部src和新增夹具覆盖、内部上下文排除、证据链接与diff检查通过。Semifold变更集macos-package-compression。Windows原生/PowerShell门禁未执行；仅本地提交，不推送或发布。
