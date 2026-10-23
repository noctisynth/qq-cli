# macOS 状态发布与持久化顺序

文档编号：CHECK-080。状态：Native storage verified。日期：2026-10-24。
关联：SPEC-008、TASK-006。起点：f153fad。

## 实现

此前atomic_json只同步临时文件，重命名后未同步目录；References新条目的目录项也未同步。macOS使用锁定rustix 1.1.5的安全封装，未新增原创unsafe边界。
目录逐层openat(NOFollow/Directory)，新目录mkdirat0700并同步父项；临时JSON和引用文件独占创建0600。JSON文件F_FULLFSYNC后相对同一目录renameat，再fsync目录并F_FULLFSYNC文件；签名引用写入后也在返回token前同步目录和文件。目录设备/inode不再匹配原路径时拒绝完成。
失败可能发生在文件已发布之后；调用方收到错误，不可据此执行后续真实操作。临时文件清理不会在发布成功后再次unlink旧临时名字。

## 验证

五项测试覆盖：JSON原子替换及0600、序列化失败保全原内容并清理临时文件、目录替换拒绝且不写替代目录、祖先符号链接拒绝、链接目标保全、签名引用私有文件可验证、新目录0700且不修改原父目录权限、已有文件不能当目录。补充在Serialize回调中替换目录，确认atomic_json拒绝发布、两侧原内容保持且临时文件清理。
scripts/check-macos.sh的fmt、Clippy、232项普通测试及arm64 release构建通过；随后新增的目录替换竞态测试及五项持久化定向测试通过，并复核all-target Clippy。源码包260项白名单及源码覆盖、git diff --check和Semifold status通过，变更集macos-state-durability。未执行断电测试、Windows原生回归或真实QQ操作。已有目录属主/ACL、读取路径、锁文件身份、跨CLI恢复和完整进程树停止仍未验收；Keychain待授权清理未重试。
