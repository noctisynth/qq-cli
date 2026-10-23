# macOS 副本签名权益

文档编号：CHECK-101。日期：2026-10-24。状态：合成签名权益及macOS离线门禁通过。
关联：SPEC-008、TASK-006。起点：a485276。

只读观察本机安装包QQ 7.0.2-53644/build 53644（package SHA-256 a4580685856ac9e62e4b04d83c8865c452626ed8bda2da9aca4403b3a89d2b27）的嵌套应用布局，确认不止主QQ与四个Electron helper；不能将同一权益递归应用于所有子应用/Framework。命令探测还发现codesign -d --entitlements -返回人类可读的字典格式，旧:-及强制XML导出可能返回空内容并警告XML权益无效。早期按XML尝试解析失败/空输出不构成“无权益”证据。生产验收因此使用系统签名信息，不依赖该输出格式。

新增Policy的Main/Electron/Native三种完整模板，分别含三个、两个或零个固定布尔权益。原生边界通过SDK SecCodeCopySigningInformation（kSecCSSigningInformation）获取flags和系统识别的权益CFDictionary；静态CF key和返回值按所有权规则管理，全部值类型检查。只接受ad-hoc+hardened runtime及精确权益集合。SDK接口声明依据本机Security.framework公开SecCode.h；没有新增依赖或调用私钥/Keychain条目接口。

Policy.verify先用Security Framework严格验证资源/嵌套代码/所有架构且禁止网络，再核验本机arm64权益。前后验证规范路径与节点元数据和截止。此处不是官方源签名验证，不接管普通QQ，不授予副本所有权。内部FFI精确开放在新增macos_signing_info，结构门禁改为九个指定原生模块；没有扩大其他模块的unsafe范围。

新测试仅复制并签名原创Rust测试程序，不执行这些已重签名副本。实际codesign生成DER权益及签名：三个角色通过；拒绝缺少主进程所需权益、额外App Sandbox、false、字符串true、缺少runtime、签名后改代码字节及角色不匹配。无签名内容、符号链接、相对路径及已截止也拒绝。并未把这一测试提升为真实QQ的签名/运行兼容证据。

后续仍需明确全部嵌套代码的角色与签名顺序，接合受控副本能力、签名进程隔离、整树差异核验、失败持久恢复及真实授权回归。同步系统校验仍无绝对墙钟上限，所有架构有效性不等于各架构权益均已匹配；精确策略只覆盖原生arm64。生产签名器未实现，本阶段只提供固定输入及其只读验收。

完整macOS门禁通过：22项Node、fmt、Clippy、306项Rust通过（2项忽略）及arm64 release构建。最初Clippy指出一个不必要借用，修复后完整重跑通过。源码包298项、全部src/新增测试覆盖及内部私人材料排除、证据链接、diff和Semifold status通过。新增unsafe声明的精确正则及九处数量以本机脚本复核；PowerShell和Windows原生门禁未执行，不以该复核替代。阶段变更集macos-signing-policy，仅本地提交，不推送或发布。
