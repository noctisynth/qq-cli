# 固定输入与实例装配

文档编号：CHECK-115。日期：2026-10-24。关联：SPEC-008、TASK-006、CHECK-110/112/113/114。起点：f4f22d9。

新增runtime/macos_assembly::Plan，在原生命周期锁和短状态事务下读取唯一prepared实例/固定Framework安装，完整i1文件校验及Source重核后生成输入摘要和Target。摘要覆盖实例/安装、固定来源与来源路径、完整原树、角色/精确权益、适配器/改写字节、目的路径、环境和参数；不接收用户自报的完整input_hash。Plan不可序列化恢复，前后复核原状态锁身份和快照。

部署接入现有d2日志：只在新Pending回调中顺序发布适配器、隔离复制、加固、入口改写、副本计划及受监管签名/校验/同步；最后完整复核来源、安装和状态，交由日志内核生成Complete。效果guard同时核对实际适配文件。错误保留部分现场并维持Unknown/Pending，既有记录不自动重放。Assembly低层组合不提供来源认证或持久意图；生产代码通过Plan接合。

自动合成测试验证输入摘要绑定各类字段、排序环境的稳定性，以及失去guard、不同目标根、外来AdapterPlan/PackageEdit在发布前拒绝。独立原创宿主注册复制、签名监管、权益和来源工作进程；只对其自身合成app进行签名、复制和装配，从未执行被复制的应用。成功取得d2 Complete、只读恢复及再次调用不执行回调；在适配器发布后注入失败，保留适配器而不生成QQ.app，取得Unknown，恢复/再次调用同样不重放；原始合成来源保全。该synthetic模式已纳入scripts/check-macos.sh，临时宿主与现场由专用临时目录清理。

tests/probes/macos_assembly.rs另提供source模式：QQ仅从固定只读DMG观察，Framework固定ZIP导入全新临时合成状态（内存测试密钥，无Keychain），构造真实来源Plan；绝不调用真实QQ的deploy或start。其结果单独记录，不混入合成签名正例。真实副本装配/GUI/OneBot、正式CLI路由、失败现场清理及启动生命周期未由此验收。

固定资产无写入正例通过：source模式认证官网原树、导入固定Framework v4.18.41并成功构造Plan；adapter、QQ.app及d2索引均未创建。修改合成实例端点后，只读Plan.revalidate拒绝旧快照。未调用该真实来源Plan的deploy。临时状态已清理，DMG正常卸载。

恢复分支补强：已有Complete跳过原效果回调，但Plan返回完成前仍验证Framework/i1及适配组。独立合成测试覆盖适配组未发布、Framework内容修改/额外文件、适配入口篡改和错误签名密钥；d2 Complete不被当作外部文件永久完整的证明。

最终完整macOS门禁通过：22项Node、fmt、Clippy、346项Rust通过/2项忽略、独立合成装配宿主两种结果及arm64 release。源码包322项覆盖全部源码且排除内部/私人材料，受影响上下文链接/私人路径与git diff --check通过。Semifold变更集macos-fixed-assembly，status通过；Windows原生门禁未运行，仅本地提交、不推送发布。
