# macOS 嵌套签名计划

文档编号：CHECK-102。日期：2026-10-24。状态：合成计划、实际布局只读探测及macOS离线门禁通过。
关联：SPEC-008、TASK-006。起点：7ce0af6。

Inventory.signing_plan消费显式审查输入但不消费写能力；完整原树前后重核，计划绑定根与指纹。逐个普通文件通过保留目录FD检查Mach-O头部，复用已有arm64 thin/fat解析并返回文件类型。不会按扩展名/执行位漏掉native载荷。全部code bundle目录与角色输入一一匹配；原描述符有界读取Info.plist并核对原摘要/长度/stamp，再交CFPropertyList解析；通用SecCodeCopySigningInformation读取签名系统主文件，可选secured Info仅作额外交叉比对，校验预期标识/执行名、主文件在本bundle内且属于原清单，无重复认领。主程序和Framework分别要求正确Mach-O类型；剩余native文件仅Native策略。目标按路径深度内部优先、根最后，不使用生产codesign --deep。

合成目录包含根应用、嵌套Electron helper、versioned Framework及链接、独立plugin.node。仅原创测试二进制和原创C动态库；初始夹具用系统codesign建立来源元数据，不执行这些副本。早期依赖secured Info的实现先在未封印夹具上失败，签名夹具成功；随后真实安装的47个bundle均缺少这一可选字段，推翻该假设。最终采用原清单约束的Info原字节解析，同时比对系统主文件；补未封印夹具成功、超限Info拒绝，说明计划不依赖该可选字段，也不代表官方来源认证。另验证资源改动导致旧封印失效时仍能进行重签名前元数据规划。检查目标覆盖一次、Framework链接不重复、根最后、Native叶及Main角色正确；清单缺项/重复、错误标识/执行名、未知应用、已截止、越界角色路径、嵌套Main、树变化和新指纹拒绝。

依据SDK SecCode.h补强CHECK-101：entitlements字典缺失也可能表示非字典权益，只有字典和原始blob均缺省才允许Native无权益。现有签名正反例保持，并补实际无entitlements参数的Native签名。签名信息函数提取共用CF所有权/类型检查，没有新增unsafe许可模块或依赖。

只读预调查当前QQ 7.0.2-53644/build 53644发现28个应用和19个Framework、89个同时含x86_64/arm64的Mach-O；未由此扩大Intel运行支持。otool观察主程序/helper没有直接QQNT依赖，不能靠LC_LOAD_DYLIB或文件名自动判定Electron权益。真实布局探测的嵌套角色统一以Native占位，只核验元数据/覆盖，不是生产角色清单、权限审查或真实运行验收。

未复制、重签名或启动真实QQ。生产签名执行器、固定版本真实角色清单、原生依赖兼容、操作隔离、整树签名后差异及失败持久恢复仍未完成。资源型/未知布局bundle不被悄悄略过，必须后续明确支持组合；同步元数据/文件系统调用没有绝对墙钟保证。

真实安装只读探测最终通过：3578个节点的Inventory，47个显式bundle元数据项，89个签名目标，其中47个bundle目标、42个独立native目标；创建计划后的整树重新核验一致。仅检验布局/覆盖，嵌套Native占位不当作真实权限策略已审查。第一次依赖可选secured Info的探测失败，修正并重编译最终源码后重新执行成功；没有复用失败结果或只改文档宣称通过。

最终完整门禁通过：22项Node、fmt、Clippy、310项Rust通过（2项忽略）及arm64 release构建。源码包300项、全部src和新测试覆盖、内部/私人材料排除、证据相对链接、git diff --check及Semifold status通过。阶段变更集macos-signing-plan；Windows原生/PowerShell门禁未执行。仅本地提交，不推送或发布。
