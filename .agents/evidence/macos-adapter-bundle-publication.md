# macOS 原创入口资源发布验证

文档编号：CHECK-087。状态：Synthetic bundle publication verified。日期：2026-10-24。
关联：SPEC-008、TASK-006。起点：ae5d566。

Prepared发布固定三文件到内容命名目录，底层使用目录描述符、独占文件、同步与RENAME_EXCL，已有目标不覆盖。资源只含原创适配与本次路径配置；没有复制上游、修改QQ或创建QQ进程。复核直接比较本程序的期望源码和精确文件集合。

新增4项合成测试：整组发布及0700/0600权限、三次guard、重复发布拒绝而旧内容完整；等长篡改/额外package.json/软链接/硬链接/公开文件权限拒绝；三个guard分别失败均不返回成功，改名前无最终目录、改名后失败保留可复核完整材料但不报告成功；第二个guard期间替换父目录，拒绝发布到替代目录。原3项启动准备测试继续覆盖路径与生成源码VM执行。

失败暂存目录明确标记.tmp并保留，不递归删除不再可信的路径；后续清理仍在TASK-006。以上是即时文件系统观察与同步调用验证，不是断电证明、完整ACL防护、受控QQ部署或真实启动验收。guard由后续生命周期调用者绑定原锁/实例权威，本测试用合成回调模拟失效。

完整macOS门禁通过：22项Node测试、fmt、Clippy、255项Rust测试（2项忽略）及arm64 release构建。源码包273项白名单及全部src覆盖、git diff --check、Semifold status通过；阶段变更集macos-adapter-bundle-publication。
