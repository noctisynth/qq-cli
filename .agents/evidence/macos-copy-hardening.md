# macOS 私有副本权限加固

文档编号：CHECK-098。状态：Synthetic hardening and isolated-copy integration passed。日期：2026-10-24。
关联：SPEC-008、TASK-006。起点：83054f5。

受控复制返回的Inventory现在持有原私有父目录描述符作为内存能力。普通inspect/verify_copy不能生成该能力；harden_copy消费能力，先检查截止、原父身份/UID/私有模式/无ACL、guard和完整原树，再通过持有节点描述符修改。原安装及重新扫描的私有副本均不能借此获取写权限，不将“当前UID拥有”本身作为修改原安装的许可。

本机原创临时文件/链接探测确认acl_init(0)/acl_set_fd_np可清除所指节点ACL，清除后acl_get_fd_np返回缺少ACL；链接描述符不跟随目标。扩展既有macos_acl原生边界，SDK空ACL由RAII释放，修改前要求当前UID及支持类型、普通文件单链接，修改后确认ACL缺失。没有新增unsafe模块许可。

目录0700，普通文件0600加原所有者执行位，链接模式不变。先子节点后目录；每个修改边界检查guard、原stamp/属主，普通文件完整同步、目录同步。完成后重新扫描，只接受预期模式与ACL变化，同时核对内容、链接文本、扩展属性/压缩资源叉/quarantine、标志、dev/inode、UID/GID、链接数、大小与mtime。ctime因权限修改变化，不伪造原ctime。指纹格式保持v4，返回新权限对应的指纹和能力。

18项应用清单集成测试通过，新增两项覆盖：原安装或仅扫描私有路径拒绝且未被改动；0666文件、执行文件、继承目录/文件/链接ACL被收紧，压缩内容和quarantine保全；来源仍可按原身份/内容重核；成功重复加固语义幂等；已变副本拒绝；已发生权限修改后guard失败返回错误并保留现场。补充已截止入口测试确认不调用guard、不改变副本。隔离复制整树单元测试也接上加固并保全原来源。

完整macOS门禁通过：22项Node、fmt、Clippy、301项Rust通过（2项忽略）及arm64 release构建。最终补充截止和大小/mtime核验后，两项加固测试、fmt及Clippy再次通过。源码包293项白名单及全部src覆盖通过；Semifold status和git diff --check通过。阶段变更集macos-copy-hardening。Windows/PowerShell门禁未执行。

本阶段只修改原创临时合成副本，没有复制、改写、签名或启动真实QQ。失败可能留下部分加固树，不回滚或返回成功能力。父端文件系统调用仍使用协作式截止；持久安装记录的写权限恢复、祖先权限、清理、入口部署、签名及生命周期接合仍待完成。

最终源码的arm64 release构建再次通过。仅本地提交，不推送或发布。
