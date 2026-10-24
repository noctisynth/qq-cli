# 固定官网签名角色

文档编号：CHECK-113。日期：2026-10-24。关联：SPEC-008、TASK-006、CHECK-112。起点：bbbd5d8。

固定来源为CHECK-112的官网DMG及原树指纹。macos_source/roles.rs收录实际42个bundle的相对路径/CFBundleIdentifier/CFBundleExecutable；1项Main、24项Electron、1项PluginHost、16项Native。主QQ及4个扩展宿主/各自helper的代码具有ElectronMain、QQNT.framework与动态加载入口观察；不以这些字符串作为运行时权限推断机制。其余独立原生映像由完整清单检查器枚举。

截图宿主的原签名包含allow-jit与disable-library-validation，且链接其专用原生框架；为固定清单增加PluginHost角色，明确与Electron宿主职责不同。新角色只含这两项布尔true，无unsigned-executable-memory、Sandbox、共享组或调试权益；这保留固定来源已有的两项动态代码权限，不宣称已经证明每个key的最小必要性。私有校验及签名监管worker识别plugin-host，拒绝其它任意权益。原创合成程序验证该角色签名/系统校验/监管执行，额外Main权益作为拒绝反例；不签署真实QQ。

Source.signing_plan对已认证原树调用完整计划器；源角色元数据供后续副本在其自身Inventory上重建计划，不授予任意副本所有权。固定目录清单不替代官方签名、完整原树认证、活跃复制能力或正式运行验收。

本阶段只读挂载官方DMG进行实际计划核验，未复制、重签或启动QQ；不读取会话。组件运行兼容性及正式部署/启动继续待办，不把静态清单或合成测试等同真实GUI功能验证。

实际只读生产计划通过：Source认证固定原树后，signing_plan返回75个原生目标，其中42个bundle；策略计数Main=1、Electron=24、PluginHost=1、Native=49（含16个bundle和33个独立映像），根最后。所有42个精确原封印路径另行只读确认是普通文件；未执行真实重签名变化测试。此次DMG已正常卸载。

完整macOS门禁通过：22项Node、fmt、Clippy、341项Rust通过/2项忽略、arm64 release。源码包315项覆盖全部源码且排除内部/私人材料，受影响上下文链接/私人路径与git diff --check通过。Semifold变更集macos-fixed-signing-roles，status通过；Windows原生门禁未运行，仅本地提交、不推送发布。
