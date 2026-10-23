# macOS QQ 静态应用审计

文档编号：CHECK-089。状态：Synthetic and installed-file inspection verified。日期：2026-10-24。
关联：SPEC-008、TASK-006。起点：2fef1ec。

新增qq::macos，qq::inspect在macOS arm64接受.app/Contents/MacOS/QQ路径，Windows QQ.exe原路径保留。检查三个Mach-O载荷及原入口ASAR/package/Info.plist，采用原文件描述符、边界与即时身份复核，并返回通用Application指纹。没有载入QQ模块、访问会话或启动QQ。

4项合成测试覆盖thin/fat32/fat64；Intel/错误类型/截断/错误subtype/不完整命令/越界/溢出/重复重叠slice拒绝；改变wrapper/major/ASAR/package/Info任一项改变指纹；稳定id与稳定内容指纹、忽略误导eleArch；缺文件、软硬链接、版本矛盾、改写入口和截止拒绝。载荷均为原创惰性字节，不能作为可运行程序的证明。

另用本次Rust库编译临时只读检查程序，读取已安装QQ应用文件（不运行该应用）：返回version=7.0.2-53644、build=53644、architecture=aarch64，选定应用指纹5fa6d9836ad144f7ebbc62434182fdb54cf06cbaa6e49b045a9600d6db8fb350。路径与临时程序不进入公共仓库。此前实验副本的静态Mach-O头观察显示QQ/major/wrapper的arm64 slice类型分别为2/8/6，与本实现的受检组合一致。

此处未验证Info.plist语义、完整代码签名/QQNT.framework/helper、DMG来源或部署后封印；未将原型副本登记为CLI运行。fat解析允许容器中有其它架构，仅选择arm64，不增加Intel/Rosetta产品支持。文件快照不封闭恶意同用户并发修改，真实效果前仍需完整部署权威复核。

完整macOS门禁通过：22项Node测试、fmt、Clippy、261项Rust测试（2项忽略）及arm64 release构建。源码包277项白名单及全部src覆盖、git diff --check和Semifold status通过。阶段变更集macos-qq-static-inspection。
