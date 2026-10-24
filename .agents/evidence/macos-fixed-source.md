# 固定官网 QQ 来源

文档编号：CHECK-112。日期：2026-10-24。关联：SPEC-008、TASK-006、CHECK-072。起点：6e87a0c。

重新核验本地留存的官方DMG：488,441,429字节，SHA-256 d270b78f0b210532637076532a883dc8659851b6daa92451983985ab9f3216f0，与CHECK-072一致。仅以readonly/nobrowse挂载，不启动、复制或重签QQ，不读账号数据。来源URL与固定摘要进入install/macos_source；未重新查询线上发布状态，不把原始下载摘要称为腾讯公开签发的校验值。

现有Rust inspect_original通过隔离系统校验，返回TencentDeveloperId、版本7.0.2-53644/构建53644。选定载荷指纹f5fba882516dab67090f825675d5f5afc7e05436f22231cafad45c99bcce2b51；Inventory v4共3437节点，完整树指纹b8087574a4f873ba4cb72cfc758ab57e47588f26ddf0a75e8778749799126b21。与商店版来源/指纹分别记录，不能按相同版本号互换。

新增Source生产入口先匹配完整树，再验证原签名及固定选定载荷，最后完整重读并比较原节点身份。其私有构造不接受调用者自报来源；只读Inventory没有owner。selector合成测试拒绝商店来源、相同版本不同树/载荷、不同构建及x86_64；这些合成输入仅测试选择条件，不伪造签名正例。

只读布局审计观察到42个code bundle（商店版此前为47个）；QQ宿主与helper含ElectronMain、QQNT.framework和dlopen字符串，但静态dylib命令没有QQNT依赖，因此不能仅凭静态依赖列表判断Electron角色。原权益采用codesign人类输出仅作调研；旧XML导出在当前系统不能可靠表达权益，不能将空输出当作无权益。角色映射及各组件权限需求尚未冻结，不因字符串或文件名自动授予权益。

固定来源匹配不等于下载/挂载监管、正式部署、签名角色运行兼容或启动验收。完整文件系统读取仍为协作式截止；不新增绝对墙钟或断电保证。探针及上游产物留本地，公共记录只保留摘要与方法。

真实只读正例：新增Source::inspect在固定官方DMG上通过，返回3437节点；随后Source::revalidate再次完整重核通过。运行宿主注册原始签名隔离worker；没有以同步旁路代替生产入口。

本次只读挂载已正常卸载。完整macOS门禁通过：22项Node、fmt、Clippy、341项Rust通过/2项忽略、arm64 release。源码包314项覆盖全部源码并排除内部/私人材料，受影响上下文链接/私人路径及git diff --check通过。Semifold变更集macos-fixed-qq-source，status通过；Windows原生门禁未运行，仅本地提交、不推送发布。
