# 私聊在线文件与目录传输

文档编号：CHECK-160。状态：R13六action协议/领域链路离线通过。日期：2026-10-30。
关联：RFC-0009、SPEC-009、TASK-007、CHECK-159。

来源固定NapCat v4.18.38/bbd0d2aca3d470c20b33b1d4f95123335633bf26：file/online六处理器、core/apis/online.ts、NodeIKernelMsgService.ts、types/element.ts的FileElement及services/common.ts状态。六实际载荷Schema与效应完成。原生ID区分OneBot message_id/m1/f1；接收实际要求element_id，没有示例save_path参数。发送是异步sendMsg加历史轮询，原生-1可能发生在已发送之后，明确unknown，同键不重发。

六领域入口覆盖私聊文件/目录发送、有界列表、显式接收/拒收/取消；user_id映射实际C2C目标。在线列表只投影23/30类型的名称/大小/原生消息及元素ID/时间，隐藏本地路径/UID，complete:false。所有异步输入复用CHECK-159的受管内容绑定/配额/持久影本；send路径字段分别file_path/folder_path。SDK默认目录接收需read/send/mediaDownload，原生result0只确认任务接受；取消/拒收void只确认SDK返回，无传输完成承诺。不自动审批、接收、取消、打开路径或重发。

原创tests/online.rs两测试含21种受管场景：六命令、file/dir真实影本字段与认证持久保留、原生发送超时/缺ID/账号变化、接收未知、首批截断/空/原生拒绝、读写/媒体/目标策略、raw实际peer/权限、同键缓存/变化内容冲突及结果Schema/不落原生结果。全部通过。CLI14与Skill3回归通过，streams2包含新增receive_online_file经api stream也必须read的零业务请求回归。命令清单142叶/162级help，技能仍11任务/14文件。

全targets check/clippy、fmt、architecture/contracts/package（525文件）、Skill quick_validate和diff检查通过。新增online-transfers changeset，semifold status共23项，规划0.2.0，未消费/发布。真实QQ、默认目录/传输完成/取消/拒绝效果、异步源生命周期与macOS仍未验收。完整累计离线与release门禁另登记，不以本切片替代原可靠性专项。
