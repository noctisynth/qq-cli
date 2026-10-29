# 本地自定义表情导入与源文件保留

文档编号：CHECK-154。状态：R09添加表情离线通过。日期：2026-10-30。
关联：RFC-0009、SPEC-009、TASK-007、CHECK-153。

范围固定NapCat v4.18.38/bbd0d2aca3d470c20b33b1d4f95123335633bf26的extends/CustomFace.ts及core/apis/msg.ts、NodeIKernelMsgService.ts。源码观察为add_custom_face先stat/readFile本地路径，再交SDK；不存在该action的base64/URL输入或复制完成证明。实现为原创Rust领域封装，未提交上游源码或本机材料。

emoji add显式选本地普通非空文件，复用128MiB单文件、1GiB缓存/256条及macOS签名运行前缀规则准备影本；名称/大小取实际内容，MD5交上游计算，package_id先核对JS精确整数，emoji_id保留字符串。预览只核对本地输入及参数，不创建缓存、凭据、意图或网络调用；不验证图片编码。

私有文件调用计划只允许add_custom_face，以kind/name/size/SHA256替换随机路径生成带密钥摘要，并增加调用方无法覆盖的fileBinding。普通JSON指纹保持原样；raw调用不能绕过内容绑定。同键同内容只复用持久结果，同键不同内容或raw替代冲突。权限、pending、账号、认证保留、一次原生调用、后置账号及终态沿共享顺序。业务仅收到受管影本路径，不收到调用方原路径。

源文件复制时机未知，保留元数据新增受HMAC认证的persistent；缺省false且不序列化false兼容原影本。普通media prune保留这些源文件，即使拒绝或unknown也不擅删；仍计缓存配额。篡改标志后签名拒绝，清理保持不可信现场，不靠操作名或可变ledger推断删除许可。

API单测12项通过，含接受/拒绝/中断/切号、同键影本去重、内容冲突、raw冲突、pending先于原生文件访问、持久保留及签名篡改。原创受管CLI五分支（默认/可选参数/拒绝/切号/权限拒绝）及离线预览、整数与文件大小边界通过；实际输出匹配派生Schema。附件4、缓存3、writes10、send_cli2、assets2、API3/API CLI1、CLI14、Skill3回归通过，均合成离线服务，不调用真实QQ。

all-targets Clippy、fmt、架构/契约、Skill quick_validate、源码包白名单489文件、diff检查通过。106叶命令/121帮助入口；10任务/13导出文件。Semifold仅规划17份变更集至0.2.0，未消费/发布。未重新运行完整suite或release构建，既有完整suite/Windows构建仅按CHECK-152快照解释。

本切片不证明SDK成功复制、QQ显示、图片解码/市场表情效果或macOS真实链路。全部真实效果与源文件生命周期验收仍待授权范围确定；未push或发布。
