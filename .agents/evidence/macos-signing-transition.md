# macOS 签名后的整树变化核验

文档编号：CHECK-104。日期：2026-10-24。状态：合成正反例与完整macOS门禁通过。
关联：SPEC-008、TASK-006。起点：0b5333b。

用原创C应用只执行系统签名，观察初次封印与runtime重签名之间的元数据变化：可执行文件inode、时间、内容变化，其父目录时间变化，CodeResources时间变化。未执行合成应用、未接触真实QQ。初次探测所用Python无os.listxattr，改用系统xattr只读合成夹具后成功；不把工具失败视为应用兼容性结果。

SigningPlan记录由已核验Info布局推导的精确CodeResources路径；应用为Contents/_CodeSignature，versioned Framework为实际版本目录/_CodeSignature。verify_transition要求原快照指纹与计划一致、同根及路径集合相同。仅计划映像/精确封印普通文件允许内容、大小、inode及时间变化，保留设备、属主/组、链接数、模式、ACL、标志及全部属性；其直接父目录仅允许时间变化；其余节点内容及完整stamp一致。最后调用全部原生slice载荷保全与当前树重核。不存在原CodeResources拒绝，不因目录同名放宽。

原嵌套app/helper/versioned Framework/native叶重签名正例通过；通用arm64/x86_64应用重签名也通过。新增五类反例：资源改写、新文件、文件删除、映像权限变化、Framework链接文本变化。每个反例再按计划逐目标签名，确认Policy及载荷保全仍通过，整树比较必须拒绝。说明有效签名本身不能接受额外资源或元数据变化。

此接口只读，不授予写权限，不执行签名、不验证全部Policy、不产生部署回执；生产签名器、原已封印布局之外的变化、压缩规范化、固定QQ角色清单及失败恢复仍待完成。未修改真实QQ或进行账号操作。源码包303项全部源码覆盖并排除内部/私人材料；Semifold变更集macos-signing-transition及status通过。Windows原生/PowerShell门禁未运行；仅本地提交，不推送/发布。

最终门禁：22项Node、fmt、Clippy、316项Rust通过（2项忽略）、arm64 release构建通过；证据唯一编号/相对链接与git diff --check通过。
