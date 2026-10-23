# macOS 应用 ACL 与文件系统标志核验

文档编号：CHECK-094。状态：Synthetic and installed-tree verification passed。日期：2026-10-24。
关联：SPEC-008、TASK-006。起点：4f2cb5a。

新增native/macos/acl.rs窄只读unsafe边界，接收BorrowedFd，使用系统acl_get_fd_np/acl_size/acl_copy_ext/acl_free。SDK独立分配由RAII唯一释放；输出缓冲区初始化、限64KiB，要求实际长度等于声明长度。仅将大端外部表示作域分隔摘要，不解析用户名、输出ACL主体或保存原值。ACL外部格式不假定跨平台稳定；本阶段仅macOS arm64。st_flags加入节点身份、清单比较及v3指纹，不主动清除或解释成运行许可。

依据本机SDK sys/acl.h、sys/stat.h及acl_get_fd(3)、acl_copy_ext(3)手册核对接口；未复制上游实现。第一轮合成清单因空ACL拒绝，独立原创临时文件/目录/链接探测确认：无ACL的已持有描述符返回NULL/ENOENT，有ACL返回有效外部表示；仅此无路径查找的ENOENT归为缺少ACL，其他错误仍拒绝。

新增链接描述符单元测试证明：目标新增ACL不会改变链接摘要；链接自身新增ACL会改变链接摘要。10项清单集成测试通过，其中新增文件ACL/目录继承ACL经ditto复制保留后可比对，副本新增deny或原目录移除ACL被拒绝；hidden文件标志复制保留后可比对，来源或副本单独改变标志被拒绝。没有测试自动清除实际应用ACL或系统保护标志。

真实QQ只读Rust扫描通过：3578节点、1375510638字节普通内容、5个带组/其他用户写位的普通文件、91项可枚举扩展属性，约50.24秒，预算180秒。未执行、复制或修改QQ，未记录ACL主体或本机v3指纹。此结果证明当前安装树可被新清单检查，不证明正式副本部署或完整来源认证。

完整macOS门禁通过：22项Node、fmt、Clippy、283项Rust通过（2项忽略）及arm64 release构建。源码包285项白名单及全部src覆盖通过。结构脚本更新为6个精确原生边界，新增ACL声明正则与计数检查通过；本机无pwsh，未执行PowerShell门禁或Windows回归。Semifold status及git diff --check通过；阶段变更集macos-bundle-acl。

剩余：默认xattr接口未枚举的压缩等元数据、复制标志策略、受控复制、私有目录及ACL/模式加固、签名和启动。保留CHECK-093历史范围，本阶段不声称完整元数据或生产部署完成。
