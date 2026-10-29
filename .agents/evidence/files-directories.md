# 文件目录上传与来源受检链接

文档编号：CHECK-157。状态：R10十四action协议及十一领域命令离线通过。日期：2026-10-30。
关联：RFC-0009、SPEC-009、TASK-007、CHECK-154、CHECK-156。

固定NapCat v4.18.38/bbd0d2aca3d470c20b33b1d4f95123335633bf26的go-cqhttp文件九处理器、file两URL及extends移动/传输/重命名三处理器、helper/data.ts、common/file-uuid.ts与Group/Msg/RichMedia接口已核对。十四实际载荷派生Schema与效应/目标分类；download_file写入后端临时文件，保留显式底层api工具入口及完整headers/base64/url参数，不把返回路径当本机可读权威。

file info保留native计数/空间且quotaVerified:false（上游硬编码10000及缺失时0/10GiB占位）。file list根/显式folder首批按file_count有界查询，合计本地limit、scan/response预算、逐项group_id复核，complete:false；无隐式递归/后续分页。列表ID为NapCat内存缓存别名，非持久文件引用；上传nativeFileId单列，不混同别名。

mkdir/rmdir/remove/move/rename/transfer构造固定实际载荷；不把trans_group_file称跨群转发。Packet的group Number转换拒绝非精确整数；明确result/retCode/failFileIdList及ok:false拒绝，缺必需ok为unknown。SDK未定义的创建目录结果只按OneBot接受说明。folder与folder_id等裸别名规则如实描述，领域选显式字段。

file upload支持private/group，私聊拒绝folder，upload_file固定true，省略name取真实basename；受管影本复用128MiB/缓存配额/运行前缀、原键内容绑定与pending/身份/认证保留/一次请求顺序。file_id允许null，缺/非法类型为unknown；首次只投影nativeFileId，缓存不读原结果，ledger无路径/文件正文/原生ID。

file url只返回明确群链接；private-url先f1验证并重读原消息，必须为私聊file及opaque别名，再在同账号/源目标调用。read/mediaDownload和群/私聊targets限制均生效，无自动URL访问。引用推导实例先于无关凭据；多实例/无关缺凭据环境仍定位原实例。裸get_private_file_url不能凭额外user_id证明来源，受限targets拒绝。

原创受管CLI27分支及离线预览覆盖：空间/根/目录/合计截断/错群，建/删目录与原生错误，删文件失败列表，移/改/传及false/missing-ok，群/私聊上传、null/缺ID/切号、群URL与三类策略拒绝，私聊f1正常/来源变化/media拒绝。实际结果符合严格派生Schema；未知/拒绝/接受缓存无第二次原生调用，敏感内容不入ledger。API13单测、files2、CLI14、Skill3、media_cli1、writes10/send_cli2及album2/social2/emoji_add2回归通过。全部合成loopback/自有进程，未操作真实QQ。

all-targets Clippy、fmt、架构/契约、Skill quick_validate、源码包白名单505文件及diff通过；126叶/144帮助入口，11任务/14导出文件。Semifold仅规划20份变更集至0.2.0，未消费/发布；未重跑完整suite/release，旧完整验证按CHECK-152快照。真实Packet能力、文件目录效果/传输完成/私聊源缓存生命周期与macOS另验；R16/R17保持暂缓。
