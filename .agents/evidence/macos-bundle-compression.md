# macOS 隐藏压缩属性与资源叉分块核验

文档编号：CHECK-095。状态：Synthetic and installed-tree verification passed。日期：2026-10-24。
关联：SPEC-008、TASK-006。起点：0f3c2a6。

根据本机SDK sys/xattr.h及listxattr(2)/getxattr(2)核对XATTR_SHOWCOMPRESSION。新增native/macos/xattr.rs窄只读unsafe边界，通过借用描述符、CStr名称和有界可写切片调用系统list/get；固定启用SHOWCOMPRESSION，不启用NOSECURITY或其它权限绕过。空切片用NULL查询长度，读取资源叉允许position偏移，不提供属性写入方法。清单指纹域升级到v4；不声称系统未授权暴露的属性可被完整枚举。

原创临时压缩文件验证默认枚举隐藏com.apple.decmpfs，SHOWCOMPRESSION可见；系统读取获得预期逻辑内容。首次尝试用ditto压缩普通同卷文件没有实际设置压缩位，因此不能作为压缩夹具证据。正式测试使用原创3000字节内容的两种zlib编码及decmpfs头，验证UF_COMPRESSED、默认不可见和完整逻辑内容；系统ditto复制后清单通过，同逻辑内容/同压缩标志但不同压缩表示的副本被拒绝。

首轮真实应用清单在约3.54秒因资源叉超出原16MiB上限拒绝。独立只读尺寸调查识别471项属性，名称和值共617331430字节，6个资源叉超过16MiB，最大182797887字节；没有属性枚举/长度读取失败。实现随后将资源叉改为64KiB分块哈希、2GiB单项/4GiB全树预算，每块检查截止和返回长度，最后复核总长；普通属性仍保持16MiB单值和64MiB累计名称/普通值预算。超过16MiB的原创资源叉可以比较，独立副本仅资源叉末尾被改动时拒绝，来源重核仍通过；并非只靠原inode时间戳发现变化。

最终真实QQ只读Rust清单通过：3578节点、1375510638字节普通内容、5个带组/其他用户写位的普通文件、471项属性，耗时约73.26秒，预算180秒。未复制、执行或修改真实QQ，不记录属性值/账号/本机指纹。此证据扩展CHECK-093/094的历史枚举范围，不把扫描通过视为完整应用部署或运行支持。

11项应用清单集成测试通过。完整macOS门禁：22项Node、fmt、Clippy及284项Rust通过（2项忽略）；最终强化的资源叉副本尾部篡改测试另行通过，并补最终Clippy和fmt。源码包286项白名单及全部src覆盖通过。结构脚本更新为7个精确原生边界，新增声明正则/计数检查通过；本机无pwsh，未执行PowerShell门禁或Windows回归。阶段变更集macos-bundle-compression。

下一步是生产复制标志、受控暂存/发布及ACL/模式加固，再接合副本入口改写、签名和启动。压缩核验不意味着这些操作已经实现。

最终arm64 release构建通过；Semifold status与git diff --check通过。仅本地提交，不推送或发布。
