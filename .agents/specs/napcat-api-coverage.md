# NapCat 必要能力与直接 API 契约

文档编号：SPEC-009。状态：Active，需求已确定，实现/验收待完成。更新：2026-10-27。
依据：用户2026-10-27明确范围及RFC-0009。关联：SPEC-001、SPEC-004、PLAN-001、TASK-001、TASK-002、TASK-007、RFC-0006。

2026-10-28 rebase对齐：运行/平台机制分别引用SPEC-006/008，类型契约引用SPEC-007；本专题编号为SPEC-009，避免覆盖远端既有文档。固定0.1.0/4.18.38审计仅作范围基线，不回退远端的版本、macOS支持或既有验收；直接调用复用各平台已验证的归属能力（Windows Job、macOS对应守卫）。

## 范围、来源与事实边界

本规范扩展原产品必要范围，旧“不提供raw action、群管理”条款由RFC-0009替代。任务式命令与直接API均为必要能力；除下列整组排除项外，不把审计缺口降为可选增强。
固定上游NapCat v4.18.38提交bbd0d2aca3d470c20b33b1d4f95123335633bf26；检索2026-10-27。
- 注册来源：https://github.com/NapNeko/NapCatQQ/blob/bbd0d2aca3d470c20b33b1d4f95123335633bf26/packages/napcat-onebot/action/index.ts
- 名称来源：https://github.com/NapNeko/NapCatQQ/blob/bbd0d2aca3d470c20b33b1d4f95123335633bf26/packages/napcat-onebot/action/router.ts
- qq-cli基线：0.1.0，提交cb6070071a1f6d43aab3cc7fc71edc2db7d2f4d7；生产依据src/transport/client.rs及src/messaging、src/app调用点。
- 同日可变main的ActionName清单与固定基线相同；此观察不作为复现实验基线，也不证明所有action运行可用。

统计单位为唯一基础注册action；不重复算_async/_rate_limited后缀，兼容别名及test_download_stream各按独立注册名保留。router的test_auto_register_01、test_auto_register_02、.get_word_slices、unknown未纳入固定getAllHandlers的178项。
178项=16项当前生产调用+2项仅枚举声明+160项无调用。用户同日追加语音获取与语音转文字后，下面154项必要缺口=162项无生产调用减去8项暂缓；加原16项，共170项必要覆盖。数量不是实现任务数或完成率，部分能力/参数/格式不因接入一个action而完成。

## 保留的16项已接入基线

get_status、get_login_info、get_version_info、get_friend_list、get_group_list、get_group_info、get_group_member_list、get_recent_contact、get_msg、get_friend_msg_history、get_group_msg_history、get_forward_msg、send_private_msg、send_group_msg、get_image、get_file。

保留原任务与验收边界：联系人详情从好友列表筛选；搜索/导出复用有界历史；转发仅读取展开；发送为文本、群成员提及、图片、file与reply段；读取只归一化text/at/reply/image/file/forward；其它段unsupported。不得将本次范围更新回写成已经支持完整上游参数或格式。

## 新增必要缺口矩阵

每组都是必要范围，必须在TASK-007分别登记实现、离线与真实验收。表中upload_private_file、upload_group_file仅有Action枚举，无生产调用，属于缺口；file消息段发送不等于专用上传接口已接入。

| 范围ID | 能力组 | 基础action数 | 必要action |
|---|---|---:|---|
| R01 | 账号资料与状态 | 11 | `set_qq_profile`、`set_qq_avatar`、`set_self_longnick`、`set_online_status`、`set_diy_online_status`、`get_online_clients`、`nc_get_user_status`、`get_profile_like`、`send_like`、`_get_model_show`、`_set_model_show` |
| R02 | 联系人与好友关系 | 8 | `get_friends_with_category`、`get_unidirectional_friend_list`、`get_stranger_info`、`delete_friend`、`set_friend_remark`、`get_doubt_friends_add_request`、`set_doubt_friends_add_request`、`set_friend_add_request` |
| R03 | 群资料与成员查询 | 7 | `get_group_member_info`、`get_group_info_ex`、`get_group_detail_info`、`get_group_share_link`、`get_group_honor_info`、`get_group_shut_list`、`get_group_at_all_remain` |
| R04 | 群管理与权限 | 17 | `set_group_kick`、`set_group_ban`、`set_group_whole_ban`、`set_group_admin`、`set_group_card`、`set_group_name`、`set_group_leave`、`set_group_special_title`、`set_group_portrait`、`set_group_remark`、`set_group_kick_members`、`set_group_robot_add_option`、`set_group_add_option`、`set_group_search`、`set_group_member_invite_policy`、`set_group_member_permissions`、`set_group_new_member_history_visibility` |
| R05 | 群请求与通知查询 | 4 | `set_group_add_request`、`get_group_ignored_notifies`、`get_group_ignore_add_request`、`get_group_system_msg` |
| R06 | 消息撤回转发与已读 | 11 | `send_msg`、`delete_msg`、`send_forward_msg`、`send_group_forward_msg`、`send_private_forward_msg`、`forward_friend_single_msg`、`forward_group_single_msg`、`mark_msg_as_read`、`mark_private_msg_as_read`、`mark_group_msg_as_read`、`_mark_all_as_read` |
| R07 | 互动与卡片 | 14 | `set_msg_emoji_like`、`fetch_emoji_like`、`get_emoji_likes`、`set_input_status`、`friend_poke`、`group_poke`、`send_poke`、`ArkSharePeer`、`ArkShareGroup`、`send_group_ark_share`、`send_ark_share`、`click_inline_keyboard_button`、`get_mini_app_ark`、`.handle_quick_operation` |
| R08 | 群公告精华待办签到 | 12 | `_send_group_notice`、`_get_group_notice`、`_del_group_notice`、`get_essence_msg_list`、`set_essence_msg`、`delete_essence_msg`、`set_group_todo`、`complete_group_todo`、`cancel_group_todo`、`set_group_sign`、`send_group_sign`、`get_group_signed_list` |
| R09 | 收藏与表情管理 | 7 | `create_collection`、`get_collection_list`、`fetch_custom_face`、`fetch_custom_face_detail`、`add_custom_face`、`delete_custom_face`、`set_custom_face_desc` |
| R10 | 文件与目录 | 14 | `upload_private_file`、`upload_group_file`、`download_file`、`get_group_file_system_info`、`get_group_root_files`、`get_group_files_by_folder`、`create_group_file_folder`、`delete_group_folder`、`delete_group_file`、`get_group_file_url`、`get_private_file_url`、`move_group_file`、`trans_group_file`、`rename_group_file` |
| R11 | 流式文件传输 | 6 | `clean_stream_temp_file`、`upload_file_stream`、`download_file_stream`、`download_file_record_stream`、`download_file_image_stream`、`test_download_stream` |
| R12 | 闪传 | 8 | `create_flash_task`、`send_flash_msg`、`get_share_link`、`download_fileset`、`get_fileset_info`、`get_flash_file_list`、`get_flash_file_url`、`get_fileset_id` |
| R13 | 在线文件 | 6 | `send_online_file`、`send_online_folder`、`get_online_file_msg`、`receive_online_file`、`refuse_online_file`、`cancel_online_file` |
| R14 | 群相册 | 9 | `del_group_album_media`、`set_group_album_media_like`、`cancel_group_album_media_like`、`do_group_album_comment`、`get_group_album_media_list`、`upload_image_to_qun_album`、`upload_images_to_qun_album`、`upload_video_to_qun_album`、`get_qun_album_list` |
| R15 | 空间与频道 | 4 | `send_qzone_msg`、`delete_qzone_msg`、`get_guild_list`、`get_guild_service_profile` |
| R16 | 凭据与底层工具 | 11 | `get_cookies`、`get_csrf_token`、`get_credentials`、`get_clientkey`、`get_rkey`、`get_rkey_server`、`nc_get_rkey`、`nc_get_packet_status`、`send_packet`、`get_robot_uin_range`、`check_url_safely` |
| R17 | 上游进程与缓存控制 | 3 | `set_restart`、`clean_cache`、`bot_exit` |
| R18 | 语音获取与转文字 | 2 | `get_record`、`fetch_ptt_text` |

上表R01至R18共154项。账号资料包含昵称、头像、签名；群管理包含群名而非QQ皮肤主题。兼容别名、上游测试入口与纯底层工具无需机械创建同义领域命令，可由直接API承担，但必须逐项列明路由、效应和验证状态。
其余用户业务需稳定的领域任务入口与参数/引用/输出契约；api call可先解除专用命令缺口，不据此勾选该组领域任务或完整交付。别名与可替代后端不得重复计算用户能力。

## 暂不纳入必要实现的8项

用户2026-10-27补充将get_record、fetch_ptt_text移入R18必要范围；剩余暂缓：can_send_image、can_send_record、ocr_image、.ocr_image、translate_en2zh、get_ai_record、get_ai_characters、send_group_ai_record。
can_send_image虽属图片探测，上一轮统计位于此组；本轮按整组排除保持一致，不暗中增加需求。暂缓不等于禁止用户通过通用api入口显式调用，上述调用不因此成为已验收领域功能。

固定公开注册表未发现QQ主题/皮肤/聊天背景与建群action；不编造实现，不以内部QQ方法声明替代公开接口。未来新公开action无需CLI名单升级即可通过直接入口尝试，但仍受所选实例的兼容版本、权限与预算约束。

## 联系人及好友关系用户链路（R02）

保持contact list/get原好友范围；新增profile USER [--no-cache]读取陌生人资料，categories查询分类好友快照，unilateral查询单向好友，requests [--limit]查询可疑申请。列表/嵌套行受scan_limit及响应预算约束，不能将上游单向列表固定99条或可疑申请count当全量保证。领域结果仅投影核对过的字段，忽略扩展任意数据；未知或缺字段不伪造成默认事实，陌生资料user_id须与请求匹配。受限目标下集合只返回允许的用户；没有可核对用户标识的申请不暴露。
contact remark USER TEXT允许清空；remove USER [--block] [--both]明确选择删除附加效应；respond FLAG --approve true|false [--remark TEXT]处理普通好友申请，accept-doubt FLAG仅接受可疑申请。可疑接口approve虽声明布尔却未用于处理器分支，不能提供虚假拒绝。申请FLAG保留原字符串，不按QQ号解析；受限目标无法从原始FLAG证明时拒绝调用。所有写入复用api同键域，预览省略正文/flag，领域回执不持久敏感数据；delete_friend的valid:false与原生result非零均为拒绝。
R01同属用户链路的contact like USER [--times]、status USER、likes [USER]按固定源码实现；用户状态依赖Packet，点赞信息只投影已核对计数/用户标识。读取他人点赞须显式--expected-uid UID（可取自同账号contact profile的uid），比对返回uid后才归属给请求用户；不匹配拒绝，防止上游未解析目标时回退自己而误归属。省略USER查询自己，start/limit限定上游批次，返回complete:false；不隐式另查资料，不自动批量点赞。

## 群查询用户链路（R03）

group member GROUP USER [--cached]读取指定成员，默认遵循上游no_cache=true，核对返回group_id/user_id；group detail GROUP补详细人数与全员禁言状态，忽略源恒空group_remark，不伪造真实备注。group extended GROUP投影原生扩展中的群主和已核对策略/显示字段，校验groupCode/resultCode，不把未知字典变为领域输出。group share GROUP [--full] [--source-id] [--additional]返回分享链接，不打开或跟随；group honors GROUP [--kind]、muted GROUP、mention-quota GROUP分别查询荣誉、禁言列表和@全体余量。
效应按固定调用路径核为read，均复用目标群限制和前后身份核对；列表/嵌套项遵循scanLimit及响应预算。荣誉底层失败及禁言监听失败会被上游折为[]，因此complete:false且不能据空结果断言无人获荣誉/无人禁言；source固定废弃strong_newbie空列表不视为功能证据。群成员接口额外依赖上游缓存已存在，失败不自动预热或重试，返回角色/时间是后端快照。头像/荣誉URL仅数据，无隐式下载。

## 群申请与通知用户链路（R05）

group requests [--source system|legacy-notices|doubt] [--group-id GROUP] [--limit N]分别映射get_group_system_msg、get_group_ignored_notifies和get_group_ignore_add_request。system向上游传count=N；legacy-notices固定普通通知50条（源码没有忽略状态过滤），doubt固定可疑通知10条且仅type=7；本地limit只约束返回条数，不扩大固定批次。邀请兼容字段InvitedRequest与invited_requests只计一次。结果按群过滤/目标策略过滤，标complete:false，保留来源和截断，不将名称当真实忽略语义。
群通知源码把原seq转为JavaScript Number，超过安全整数的request_id可能已丢精度。领域输出区分observedRequestId与requestFlag：只有正安全整数才生成可供操作的flag，超界仅展示观察值，不可据其审批；原始事件flag由事件专题继续接入。
group respond FLAG --approve true|false [--reason TEXT] [--search-limit N]显式处理加入/邀请申请，单次业务请求并复用共享请求键。flag只在当前实际账号使用，不按QQ号解析；目标限制无法从flag证明时拒绝，不接受调用者附加未被上游使用的group_id来伪造目标证明。读取申请不自动处理，预览/回执不输出flag或原因。

## 消息转发（R06后续切片）

message forward MESSAGE_REF... --conversation TARGET [--merged]每次最多32条签名引用；单条默认单条转发，多条或显式merged发送合并卡片。所有来源须处于同一账号/实例/运行。默认只合并同一会话；跨会话须显式--allow-self-copy并通过自己的私聊目标权限，因为固定非Packet源可能先发送自己副本；预览/回执标maySendSelfCopies，不承诺失败时没有先前副本。不自动读取全文后重发，也不接受虚构发送者字段。首次意图内逐条重取/核对来源身份，来源和接收会话都检查目标策略，缓存不重取来源。
单条映射forward_friend_single_msg/forward_group_single_msg（无消息ID回执）；合并映射send_private_forward_msg/send_group_forward_msg，以真实节点ID构造消息，必要的空昵称/内容仅用于满足声明结构，处理器ID分支采用实际消息。源码send_msg/send_forward_msg为等价通用入口，可由裸调用补充；完整混合载荷Schema仍在格式专题核对，不把引用节点子集当完整原生Schema。
合并成功只保存有界messageId/conversation元信息，严格校验后用于原回执与request get恢复；正文、节点正文及原生完整结果不落意图。缺少/不合法消息ID和输出超限保持unknown且不重发。预览需本地签名材料，不联网/不创建意图。接受回执不等于送达证明；上游可能跳过无法读取的节点，提交引用数不证明实际卡片包含所有节点。自定义节点及完整混合格式仍另行登记。

## 消息撤回与已读（R06首段）

message recall MESSAGE_REF使用现有m1签名引用，写入前核对所选实例/运行代次、实际账号及重新获取的消息ID/时间/序号/发送者/会话；不接收猜测的短ID作为领域输入。message mark-read由--conversation CONVERSATION、--message-ref MESSAGE_REF或显式--all三选一；引用只用于定位会话，上游实际标记整个会话已读，不承诺单条/截至某条。权限受同一读/写策略约束，all在目标限制下拒绝，不把空目标数组当全局授权。
带引用的动作把签名定位元数据的带密钥摘要加入共享意图，在首次意图后、业务请求前执行重新核对；已记录请求不为核对重新读已撤回消息。裸API不能提供这份绑定证据，同键与绑定领域操作冲突而不重发。纯会话/全局已读沿普通api防重，可与相同action/载荷直接入口恢复。预览引用时只读取本地签名材料，不联网/不写意图；不把本地登记当实际身份已验证。
直接mark_msg_as_read/mark_private_msg_as_read/mark_group_msg_as_read是同一源实现，message_id优先于user_id再于group_id；仅无message_id且目标可证明时才在受限裸调用中开放，不按别名名称猜目标。带node/reply/forward或相应CQ引用的裸发送不能只证明接收群而忽略来源；需读权限，受限目标下缺少来源证明则拒绝。delete_msg及单条转发依赖运行内短ID缓存；撤回处理器未核对全部原生返回，回执只表示上游已接受。转发领域任务及完整消息格式另分阶段实施，不据首段勾选R06全部。

## 群管理用户链路（R04）

固定源码的17项写入统一作用于group_id，复用目标限制/实际身份与共享请求键。领域入口group rename/card/admin/ban/ban-all/kick/kick-many/leave/dismiss/title/avatar/remark/robot-policy/join-policy/search-policy/invite-policy/member-permissions/history-visibility；leave与dismiss共享set_group_leave但明确区分退出和解散。禁言必须显式duration，0解除；开关要求显式true/false，省略的可选权限不修改，不根据缓存角色代替上游权限判断。头像沿明确本地输入快照，最多8MiB。
群搜索/机器人/加群数值选项仅暴露固定源码字段，不猜测未知代码含义。title依赖上游Packet，未提供duration字段。批量踢人单次最多100个用户，上游可能过滤未能解析UID的用户；回执只声明已接受，不声称每个成员已移除。管理员/踢人/退出源码不核对全部底层结果，同样不以null替代最终效果证明。CLI只发一次，上游群名处理器自身的1287重试属于固定上游行为。
每项payload由实际序列化协议类型派生Schema；领域输出为严格operation/groupId/requestId/outcome/deduplicated回执，dry-run不读取配置/联网/保存意图，也不回显备注、答案、图片或其它敏感载荷。现有group list/get/members保持原读取契约。

## 账号资料用户链路（R01首个领域切片）

目标命令为account set（--nickname/--signature/--sex，sex需nickname）、account avatar FILE、account presence（--status/--ext-status/--battery-status）及account status-text TEXT（--face-id/--face-type）。写操作共用request-id、dry-run与总截止；一条命令只对应一个上游动作，不把多步部分成功藏在“全部修改”里。只有签名时走set_self_longnick；含昵称时走set_qq_profile，保留上游未指定资料的语义。
头像读取明确给出的普通文件，最多8MiB，以不可变字节快照编码为上游支持的base64输入；不要求NapCat访问CLI原路径，不隐式下载URL或改图。dry-run仅返回字段/大小摘要，不回显头像或签名。领域结果为严格DTO，只表示请求accepted/unknown，不把回执当客户端界面已刷新。
复用直接调用的受检传输与请求键/摘要，但领域输出不透传原生对象；相同action与参数从直接/领域入口复用同一键，异参冲突。上游返回明确的非零data.result时不能伪造资料设置成功；原始接口结果保真仅限显式api命令。
固定源（2026-10-29核对，提交仍为本页固定基线）：action/go-cqhttp/SetQQProfile.ts、action/extends/SetQQAvatar.ts、SetLongNick.ts、SetOnlineStatus.ts、SetDiyOnlineStatus.ts。数据Schema取实际序列化的协议payload类型，不复制维护手写Schema。SetQQProfile/SetLongNick返回通用原生结果，具体成功字段须按此源约束解释。
R01同组的固定兼容限制：action/go-cqhttp/GetOnlineClient.ts虽触发设备查询但最终恒返回空数组，GoCQHTTPGetModelShow.ts返回固定napcat，GoCQHTTPSetModelShow.ts为空处理器。保留显式api访问及准确描述，不据空成功承诺在线设备/机型修改已实现；真实业务能力缺口由TASK-007单列，不修改NapCat绕过。

## 消息互动用户链路（R07首段）

固定提交的msg/SetMsgEmojiLike.ts、extends/FetchEmojiLike.ts、GetEmojiLikes.ts、SetInputStatus.ts、packet/SendPoke.ts及core/apis/msg.ts、services/NodeIKernelMsgService.ts已核对。message react M1 --emoji-id ID --enabled true|false为显式写入，首次意图内核对m1来源，要求read/send及来源目标；该设置接口仅支持上游缓存的32位短消息ID，不将长ID转换为短ID或猜测。返回原生类型为unknown，只承诺OneBot已接受，不保证显示效果。
message reactions M1 --emoji-id ID [--emoji-type TYPE] --limit N使用get_emoji_likes的有界原生分页，先核对原消息；长ID只允许原定位为群消息，并从签名来源提供group_id。缺省emoji_type沿源的ID长度推断。该层忽略底层result并可能折为空/部分结果，因此complete:false；user_id实际上取tinyId，输出reactorId而不冒充可用QQ号。fetch_emoji_like提供另一底层分页结构，直接API可访问并核对其Schema，不额外创建同义领域命令。
contact poke USER、group poke GROUP USER分别使用friend_poke/group_poke，send_poke为同处理器兼容入口，直接API可用。均为有键写任务且依赖Packet；群戳授权群目标，私聊戳只使用同一用户作为peer与target，不自动展开到其它会话。源将ID转JS数字，领域拒绝超安全整数而不戳错误目标；裸接口受限目标下同样不得用不同target_id或数字舍入绕过。
contact input-status USER --event-type CODE显式发送上游输入状态事件，CODE保留公开数值含义，不在源码未定义时虚构状态名称；无自动定时发送。所有写命令沿现有离线预览/共享意图与严格元信息回执；真实效果另验收。卡片、内联按钮、小程序与快速操作复合任务仍在后续范围。

R07卡片/按钮补充：固定extends/ShareContact.ts表明ArkSharePeer/send_ark_share和ArkShareGroup/send_group_ark_share均只读取生成内容，按read归类，名称含send也不发送。contact card USER [--phone-number TEXT]投影原生result=0的arkMsg；group card-json GROUP取ArkShareGroup的原生字符串。严格cardJson字符串仅返回本次调用，最终类型化包装也按配置输出预算核对，URL不跟随，不将查询回执称消息发送。直接SharePeer在group_id非空时优先群分支，目标提取必须按此核对。组别名及native mini-app构造工具由直接API提供；get_mini_app_ark两种模板/自定义参数按实际类型派生Schema，返回仍是显式API动态边界，不为通用Ark对象放宽领域JSON。它只是Packet生成工具，后续发送按格式专题实现。
message click M1 --bot-app-id ID --button-id ID [--callback-data TEXT] --request-id KEY只处理签名群消息，在首次意图内重读并核对原消息，msg_seq取已绑定真实序列号，不采用上游危险的默认10086。callback内容只当数据，要求明确写权限，不自动点击聊天按钮；回执只证明上游接受，不能证明机器人业务处理完成。
.handle_quick_operation属于兼容复合入口，仅直接API访问：源立即返回并吞内部错误，不能证明所有删除/踢人/禁言/回复/审批完成。效应为write，含源读取，普通成功确认仍记unknown且不重试；受限目标不能仅信任context声明。完整mixed消息Schema完成前parametersKnown:false，不为此伪造Schema或新造重复领域工作流。

## 群精华、待办与签到（R08首段）

固定group/SetEssenceMsg.ts、DelEssenceMsg.ts、packet/BaseGroupTodoAction.ts及Set/Complete/CancelGroupTodo、extends/SetGroupSign.ts、GetGroupSignedList.ts已核对。message essence M1 --enabled true|false写入设置/移出精华，源只查短消息缓存，领域要求签名群来源及32位短ID；移出虽支持seq/random/group元组和独立essence缓存，本入口仍以核对过的m1为准。两动作首次意图内重读来源，要求read/send与源群许可，不把精华列表伪造的缓存ID当真实m1。
message todo M1 --state open|complete|cancel分别映射三个待办动作，群号必须可经JS数字精确表示；message_seq取签名消息序列并在意图内重读核对，不信任调用方提供另一群号或未绑定序列。原始接口只有缺message_seq时才会从message_id读取消息，且该缓存源可能属于另一会话，受限raw只允许无来源读取的明确seq路径并核对实际群号。
group sign GROUP显式签到，send_group_sign为同处理器别名；依赖Packet、JS安全群号和写键，不自动每日执行。group signed GROUP --limit N查询当日上游首批签到信息，有界投影并complete:false。原生只取page[0]，rank可能为小数；time源未确认单位，输出nativeTime原数值，不声明秒或本地日期。原生QQ号先转JS数字，领域拒绝不精确范围，避免归属错误。所有写入按OneBot接受记录，不替代真实群效果；剩余公告与精华列表另做用户链路和验收。

R08公告与精华列表补充：固定go-cqhttp/SendGroupNotice.ts、group/GetGroupNotice.ts、DelGroupNotice.ts、GetGroupEssence.ts及core/group与WebAPI类型核对。group notices GROUP --limit N投影去重后的feeds/inst有界快照，返回正文、图片ID/尺寸和已存在的阅读/新成员/置顶字段，忽略任意settings；complete:false，不跟随图片ID/URL。group notice GROUP [--content TEXT] [--image FILE] --request-id KEY发布（至少明确一种输入），支持源的pinned/confirm-required/notice-type/show-edit-card/tip-window-type参数；图像从调用方选定8MiB内普通文件快照为base64，不转发本地路径或自动下载URL。预览不联网/建意图，发布只发一次OneBot任务；源先上传图片再发公告，失败可能留下上传资源，不自动清理或换键重发。group notice-delete GROUP NOTICE_ID显式删除，均沿群目标许可与共享键域。
group essence-list GROUP --limit N读取有界精华快照，投影操作人、原生序列/random/缓存ID及已支持的内容段；上游可能为无法获取的原消息制造缓存ID，不能据此签发m1或可下载f1。无原消息定位的内容只显示，不扩展下载/转发权威。group essence-remove GROUP --sequence SEQ --random RANDOM使用完整原生元组，允许在原消息已不可读取时明确移出；核对有符号整数可经上游parseInt精确表达，作用域由实际group_id决定，不偷换成消息定位。此路径与m1精华动作共享同一delete_essence_msg键域；其它原生字段不同同键冲突，不能重复副作用。完整字段/效果及新成员公告范围仍需授权真实验收。

## 收藏与自定义表情（R09）

固定extends/CreateCollection.ts、GetCollectionList.ts、FetchCustomFace.ts、CustomFace.ts及core/apis/collection.ts、apis/msg.ts、Collection/Msg服务已核对。favorite create --brief TEXT [--raw-data TEXT|--raw-file FILE|--stdin] --request-id KEY将明确UTF-8原始字符串交给当前账号的收藏创建；不解析或执行内容，不补造QQ富文本格式/原消息作者。输入和非媒体实际JSON载荷受1MiB预算，预览不联网/建意图；只回元信息，不持久原始内容。源返回unknown，OneBot接受不保证渲染或条目可用，不伪造cid。
favorite list [--category N] --limit N读取首批收藏摘要，时间/ID保留原生字符串，不把摘要当正文；result/errCode任一存在的非零状态均拒绝。hasMore如实投影，complete:false、bodyAvailable:false；只投影已核对的richMediaSummary brief/title，任意其它summary不变成公共JSON。收藏可能含跨会话摘要且缺可靠来源，targets受限时拒绝读取，不能以作者字段冒充原消息作用域。
emoji list --limit N使用fetch_custom_face_detail，投影资源ID、可精确表达的数字emojiId、MD5、描述及存在的URL/原生标志，不暴露emoPath/original/thumb路径或触发OCR；fetch_custom_face是同读取业务的URL替代直接入口。源丢弃底层状态，空结果不证明无收藏表情，complete:false。URL仅数据，不打开或签发下载权威。
emoji remove RES_ID... --request-id KEY按明确资源ID删除，最多100项且非媒体JSON整体仍限1MiB；不能把MD5当可删除ID，原生md5输入虽声明但处理器忽略。emoji describe RES_ID --emoji-id ID --md5 MD5 --description TEXT --request-id KEY传递三字段定位与新描述，emojiId先核对JS精确整数（0可表达），空描述明确清空。自定义表情仓库属于当前账号资产，不从它推断聊天来源；自身读取/写入分别服从read/send与账号/实例核对，结果是接受元信息，不承诺界面已更新。
add_custom_face只接收本地路径且源会stat/readFile；其领域入口使用下述受管快照、同键内容防重及文件保留，不转发任意后端路径或以raw路由可用代替上传验收。Native长期是否复制源文件尚无证据，不提前自动清理引用文件。所有新命令保持严格领域DTO，API原生结果保真仍仅显式直接入口。

R09本地表情添加补充：emoji add FILE [--emoji-id ID] [--package-id ID] [--market] [--original true|false] --request-id KEY使用add_custom_face。源仅支持stat/readFile的本地路径；调用方路径先按既有普通文件/链接/128MiB单文件及缓存配额规则准备不可变影本，macOS沿已签名运行前缀。原生file_name/file_size由实际影本派生，MD5由上游读取影本计算，不接受伪造的尺寸/摘要；package_id的Number转换先核对精确整数，emoji_id按源保留字符串。
文件调用的私有API计划仅允许已核对的本地file字段：指纹用原参数加文件kind/name/size/SHA256替代随机缓存路径，目标与效应仍按原action/参数核对，不接受调用者覆盖。先持久pending并核对实际账号，再以同一意图哈希签名保留影本，最后只发一次业务调用；缓存请求不再次保留影本或发出请求，同键不同内容冲突。API原始路径调用与内容绑定计划不相互绕过，无法证明等价时同键冲突。
SDK未证明在接受时已复制本地源文件，因此此类a1影本在认证元数据中明确persistent，普通media prune保留；该字段缺省false兼容旧影本，持久标志也受HMAC认证，不靠可变ledger操作名决定删除。未知/拒绝后不自动删源或重发。保留文件计入原有配额，不自动绕过配额或新增不受控目录。dry-run只核对本地普通文件/大小与参数，不建立运行缓存/凭据/意图，不宣称图片格式或SDK可用；真实添加/复制生命周期仍另验收。

## 格式与事件扩展

语音专项（R18，必要）：固定提交的file/GetRecord.ts、file/GetFile.ts、msg/FetchPttText.ts、api/msg.ts及common/src/file-uuid.ts已核对。record段的file为原消息元素的缓存标识，绑定m1来源后签发f1；不把path/url当本地读取权威。media download F1 --output FILE [--format original|mp3|amr|wma|m4a|spx|ogg|wav|flac]映射get_record，默认original（out_format空串），显式格式仅允许record。它读取/下载并可转换本地缓存，与get_file相同归read且要求mediaDownload；下载仍核对来源、作用域、字节预算、原子发布，不播放。源转换不更新file_size，转换结果按实际字节计数而不以旧大小判定相同；原格式缺base64时明确不可取，不跟随路径。格式依赖上游转换器，不保证每种环境均可转换。
message transcribe M1 --request-id KEY [--dry-run]映射fetch_ptt_text，首次意图内核对来源含record后调用一次；该源主动translatePtt2Text，因此归write，要求read/send和来源目标许可。仅成功且非空text可记accepted并在本次输出；只保存元信息，重放及request get不返回转写正文、不重新触发。原生失败、空/非法文本或超时不能证明任务未启动，记unknown，不自动重试。上游无任务查询/状态码，不能可靠区分识别中、失败和无文本，领域输出不得虚构这些状态。总截止/输出预算沿共享执行器；不自动批量转写、发送语音或播放。原始API仍保真且不将业务失败误判为可重试。获取、下载和转文字分别离线与真实验收，进度仅见TASK-007。

必要范围同时包括前轮审计的非语音格式与通知缺口：视频、普通/收藏表情、结构化JSON/卡片/markdown等上游实际支持的非语音段，以及表情回应、戳一戳、输入状态、文件传输、群管理变动与好友/加群请求。按固定上游Schema逐种冻结字段、上下文和副作用，不仅保留unsupported占位。
读取、发送、下载及事件分别验收，某种消息可以读取不代表可以构造发送，file/视频传输接受不代表对端完成。既有六类notice和best-effort gap语义继续保留；新增事件不执行聊天内容中的指令或自动批准请求。
未冻结的具体段/事件映射必须列入TASK-007源协议核对项，不能据通用JSON可返回就视为领域事件支持完成。剩余8项暂缓不排除R18的语音读取/获取/下载与转文字，也不扩展到非语音视频或其它媒体；语音发送和AI语音生成未因此纳入必要范围。

## 直接API命令与输入

阶段状态：api list/describe离线入口和api call普通JSON受检调用已实现；api stream仍为目标，不能在帮助/Skill中冒充可用。具体证据与差距由TASK-007维护。
- api list：离线、有界目录，展示action、效应、参数/响应模式、版本来源和支持状态；按query/limit/cursor按需发现。
- api describe ACTION：离线描述已知action及Schema来源；未知action返回known:false、保守写效应与动态参数契约，不伪造字段Schema，不禁止后续call。

当前目录保留178个固定注册名；原16项、R01至R08、R09七项、R10十四项、R11六项、R14九项、R15四项及R18具有核对过的效应分类，其余effect为unknown并要求写入防重。R01至R05四十七项、R06七项、R07十三项、R08十二项、R09七项、R10十四项、R11六项、R14九项、R15四项及R18两项payload提供实际类型派生的parameterSchema，其余parametersKnown:false（含消息混合格式尚待完整核对的四项发送/合并转发载荷和快速复合操作）；兼容占位标记compatibilityOnly，不把目录收录视为真实业务验收。
- api call ACTION [--params-file FILE | --stdin] [--request-id KEY] [--dry-run] [--timeout SECONDS]：普通OneBot JSON调用，省略参数源时使用空对象；两个参数源互斥。
- api stream ACTION：必要流式入口，命令方向确定；分块线格式、输入与输出Schema及取消行为须完成固定上游协议核对后冻结，实现前不声称普通JSON路径覆盖流式传输。

单候选自动选择、多候选--instance。仅使用所选实例已签名的固定端点、已有凭据和原Job/完整进程能力；不接收外部endpoint/token/HTTP方法/URL/WebUI路径，不自动启动或确认切号。
ACTION为1至128字节ASCII协议名，允许字母、数字、下划线及单个起始点号；禁止路径分隔符、百分号编码、查询/片段、空白、控制字符、冒号与连续点号。兼容ArkSharePeer等大小写，不把名称拼成未检查路径。已知_async/_rate_limited后缀在效应/目标/防重分类前解析到基础名；未知名仍可调用且按未知写效应管理，不按get_推断只读。
params要求UTF-8 JSON object；保留嵌套字段、未知键、null和ID类型，不自动填入目标或改写account。拒绝重复对象键、越界数值和解析不完整；固定预算内应避免有损数值转换，语义同一JSON的键顺序不影响指纹。不得将参数值展开为shell、路径或新的请求URL。
当前普通JSON实现启用serde_json的精确数值表示，保留输入数值文本而非先转浮点；单个数值最多512字符且不得超出有限浮点范围，参数最大64层。此为CLI传输保真，不保证上游JavaScript/QQ同样保留所有数值精度；账号等ID仍优先用字符串。
默认总截止30秒，允许1至300秒；连接、身份检查、请求与响应共享截止。JSON输入上限1MiB、嵌套64层，普通上游响应/CLI输出各上限4MiB且服从更低本地配置；超限明确报错，写出请求后响应超限仍可能unknown，不能自动重发。大文件不借base64无限扩大JSON预算，走专用有界文件/流式协议。

## 效应、权限和持久请求

已知目录必须基于固定源码逐项标注只读、写入、生命周期、秘密读取、流式/多步和目标提取规则；“名称像查询”不构成只读证明。新action默认未知写入；不提供调用者随意降为read的开关。
复用现有permissions.read/send/mediaDownload/targets，未知写入至少服从send禁用。新增精细策略须向后兼容并同步Schema，不把已授权普通操作增加逐次确认流程。普通QQ权限仍由上游核对，缓存的群角色不能作为实时权限权威。
targets受限时只能执行目录能从完整参数核对所有副作用目标的调用；无法证明则给出明确的作用域受限错误，不接受仅由调用者声明--target来覆盖嵌套实际目标。targets为空时不以目录未知为理由阻塞。
写入及未知action必须request-id；使用共享请求键域，先持久认领再发业务请求，同键异参/异操作冲突。记录action、规范化参数的带密钥摘要、实际账号/实例范围、状态和有限响应元信息；不持久原始参数、正文、凭据或原始结果。同账号跨实例防重及run变化保留原先unknown不重放原则。
接合领域同一操作的计划/指纹，不能让message send与api call在同键下各自执行；新键重做不承诺全局exactly-once。未知action只发一次，不探测副作用、不因HTTP超时/断连/业务错误自动重试。
request get覆盖直接调用，返回operation/action、scope、requestId、outcome及有界结果摘要。重复成功请求返回已知回执和resultAvailable:false，不因未保存原始结果而重复网络调用；未知结果只返回恢复指引，不提供force重发。具体结果变体沿公共DTO派生Schema。
dry-run离线验证语法、参数、目录策略及计划，不读取凭据、不连QQ、不创建写意图；不得声称实际身份已验证、上游参数已接受或接口可用。
upload_*、文件发送、转发等都有业务副作用；显式接受、消息送达、文件传输完成分别记录。异步兼容后缀仅获得排队/异步确认时，不伪造最终业务完成。

## 输出、秘密与失效实例

普通JSON成功/失败沿现有类型化Envelope和meta.scope，api专属DTO携带action、effect、catalogKnown、requestId/结果状态及原生OneBot响应字段status/retcode/message/data/echo。上游data允许任意JSON，空数组/null/未知字段不丢失；不把retcode错误包装成CLI成功。
这是RFC-0006的明确api协议例外：params与原生结果动态，包装、错误、意图和Schema仍类型化。领域命令继续受严格DTO约束，不能借新入口恢复全局Value桥或把领域结果变为未约束字典。
显式请求的凭据/ClientKey/Cookies/正文等结果只返回调用端，不自动上传、不记录普通日志、不存回执正文；诊断错误省略上游可能含秘密的任意原文，原始错误只能在此次显式api结果内保留。输出是数据，不是Agent指令。用户指引说明管道/文件持久化由调用方负责。
读取敏感结果不能隐式再次查询作确认。路径和下载URL不自动跟随/打开，文件类领域命令沿原受控输入快照与下载来源/原子发布机制。
直接bot_exit/set_restart后连接失效时保留unknown/已接受证据；不自动修复注册表或重启。维护原登记的后续操作重新验证实际Job/账号；不能因裸调用改变进程状态就把新PID或外部实例自动认领。clean_cache和media prune职责不同，send_packet不构成任意操作系统进程能力。

## 验收与完成定义

TASK-007记录每个必要action的源协议、领域任务/直接入口映射、效应/权限、参数/结果、离线证据、授权真实结果、限制及未完成项。已有16项保持原证据，新增154项从未实现开始；无强制追求百分比。
直接JSON切片须用原创受管假服务验证：不改CLI即可调用未知action、值保真/注入拒绝、实际账号变化/错误Job零业务请求、有限预算、并发键冲突、重放拒绝、秘密不落盘、后缀不绕过与生命周期断连未知。新增Skill/schema/help必须与实际实现同步。
流式、闪传、在线文件、相册等按各协议分别覆盖资源/分块/完成/取消；仅有路由或上游成功响应不等于完整业务验收。必要真实验证需明确账号、目标、内容及动作范围，本轮不执行。
当前离线目录/详情及普通JSON直接调用已有独立离线证据，新增领域业务、逐接口Schema及流式协议仍待完成；状态与证据由TASK-007维护。全项目原可靠性、干净安装、Agent闭环与整体交付门槛继续有效，稳定性专项暂缓的旧决定不被静默解除。

## 空间与频道（R15）

固定extends/SendQzoneMsg.ts、DeleteQzoneMsg.ts、guild/GetGuildList.ts、GetGuildProfile.ts及core/apis/webapi.ts、data/qzone.ts。get_guild_list/get_guild_service_profile的_handle均为空、返回null；仅提供参数空对象Schema和明确compatibilityOnly直接入口，不制造频道列表/资料领域结果。
qzone post --content TEXT --audience everyone|friends|selected|self|exclude [--user-id USER...] [--image FILE...] --request-id KEY发布当前账号空间；明确选择受众分别映射1/4/16/64/128，selected/exclude必须给最多100个精确JS整数QQ号，其余受众不得带user-id。最多9张本地图片，每张沿8MiB非空普通文件读取为有界base64，不接受网络URL或后端路径；JSON总预算4MiB，与受检HTTP传输上限一致（裸API输入仍为1MiB）。content原样传递，不执行正文/图片。预览只读明确输入，不建立配置、凭据、意图或网络效果。
qzone delete TID --request-id KEY显式删除当前账号说说；不提供上游未注册的列表读取。空间受众不能映射为聊天会话，targets有本地限制时两类业务均拒绝，不凭额外group_id或target_uins伪造授权。读写权限沿既有send与共享持久意图；仅一次上游动作，图片先逐张上传再发布且非原子，拒绝/未知也可能留下图片，不能用新键自动重发。
发布返回非空tid才视为接受；返回缺失/无效为unknown，原始tid只在首次成功返回，不保存正文/图片/完整结果到ledger，缓存回执不重新联网取tid。delete原生void/null只证明上游接受删除请求。频道占位、真实空间效果、隐私受众、图片上传与macOS分别验收。

## 群相册（R14）

固定extends下九注册action的八个处理器文件、core/apis/webapi.ts及NodeIKernelAlbumService.ts、data/album.ts。album list GROUP [--attach-info TOKEN]、album media GROUP ALBUM [--attach-info TOKEN]读取一批；--limit有界投影，超过本地截断时不提供跳过未返回项的续页令牌，complete:false。相册优先SDK的name/album_id，源码示例album_name仅作兼容投影；媒体只投影处理器示例明确的media_id/url，不把media_id猜作lloc，不制造消息/下载引用。media原生result非零拒绝；缺未知列表形状明确返回协议错误。list处理器丢弃SDK状态，不能以空列表证明完整。返回URL是数据，不自动跟随。
album remove GROUP ALBUM LLOC、album like GROUP ALBUM BATCH --enabled true|false [--lloc LLOC]、album comment GROUP ALBUM LLOC --content TEXT为显式有键写任务；省略lloc的like针对整个上传批次。相册资源ID保持字符串，不冒充聊天消息ID。所有API裸路由和领域路由都核对实际group_id及本地群目标，读取用read，写入用send。
album upload GROUP ALBUM --album-name NAME --file FILE...一次上传一批图片（1至32个）；单图走单图action，多图走批量action，后者保留上游同批语义。album upload-video GROUP ALBUM FILE --album-name NAME上传视频，SDK自动首帧/默认封面。文件只取既有普通非空文件/128MiB单文件/1GiB缓存及256条配额的受管不可变影本；macOS沿签名运行前缀。私有文件计划按action固定file或files，内容指纹替代随机路径并绑定原键；pending与账号核对后认证保留全部影本，再一次原生调用。raw不能覆盖绑定；缓存同键不再保留或调用。原有emoji单文件指纹和persistent语义保持。
上游上传在await内完成文件读取/分片，领域不透传大base64或调用方路径；沿既有附件保留策略，unknown/pending不清理，已终态满足保留期后media prune才可清理。上传多图/视频是多步骤且不可回滚，超时/异常可能部分完成；回执只证明OneBot/已核对原生状态接受，不制造媒体ID、取消能力或对端完成。视频解码器与实际相册效果、分页返回形状、原生未知SDK状态及macOS仍须真实验收。

## 文件与目录（R10）

固定十四处理器、helper/data.ts、common/file-uuid.ts及core Group/Msg/RichMedia接口。file info GROUP读取群文件数量/空间（上游limit_count固定10000，空间缺失会用0/10GiB占位，不当实时精确额度）；file list GROUP [--folder ID] --limit N取根/指定目录首批，native file_count按N传递、无后继分页，complete:false。投影核对group_id、原生大小/计数、文件/文件夹名称及ID，不伪造下载引用。列表file_id为内存别名，随运行/缓存失效，需重新查询；上传返回nativeFileId与此别名不等同。
file mkdir GROUP NAME、rmdir GROUP FOLDER、remove GROUP FILE_ID、move GROUP FILE_ID --from DIR --to DIR、rename GROUP FILE_ID NAME --folder DIR、transfer GROUP FILE_ID均为显式有键写任务。folder/folder_id和folder_name/name的裸别名优先级沿源码，不把trans_group_file描述成跨群消息转发。Packet移动/重命名/群文件URL会将group_id转Number，先核对JS精确整数；其余保持String。
file upload CONVERSATION FILE [--name NAME] [--folder ID] --request-id KEY支持私聊/群文件，私聊不接受folder；名字省略取源文件名，目录字段显式映射，upload_file恒true。只向上游传受管128MiB非空普通文件影本、固定内容指纹与fileBinding；沿现有缓存配额/macOS运行前缀、pending/账号/保留/一次调用顺序及终态清理策略。仅首次结果投影可空nativeFileId，缓存无原始结果；接受不表示对端下载完成。
file url GROUP FILE_ID获取明确群文件链接；file private-url MEDIA_REF先通过既有f1来源重新读取/核对原消息，须为私聊file且opaque_id，随后在同来源账号/目标下调用get_private_file_url。引用推导实例先于无关凭据访问；两种链接均需read/mediaDownload许可，返回URL只是数据，不自动联网读取。裸私聊接口从不可解码别名恢复peer/msgId，targets受限无法证明来源时拒绝，不信任额外user_id。
download_file是写入后端临时文件的显式底层工具，仅api call提供全参数Schema与保真结果；不把路径当本机可读权威、不自动跟随URL/头或重试，受mediaDownload和有键写任务限制。普通附件落盘继续用media download的f1链路；大文件流式另见R11。原生明确result/retCode/ok失败必须拒绝；未知/异步/断连不重发，不将创建目录SDK的unknown值或文件发送accepted当最终交付。

## 流式文件与裸流协议（R11）

固定stream下六action及StreamBasic/BaseDownloadStream、OneBotAction.ts、network/http-server.ts。HTTP工具包对每个req.send发送OneBot包装JSON加CRLF空行，最后再写handle结果并结束响应；不是SSE，也不是单JSON。下载先stream/file_info，再零或多stream/file_chunk，最后response/file_complete；错误可为OneBot failed及data.type=error或未带stream的校验错误。upload_file_stream每次RPC只处理一个创建/块/完成/验证请求，分块确认type=stream并不代表文件完成，最后须独立is_complete调用得到response/file_complete。reset会清理后抛错，不当正常成功；clean_stream_temp_file删全部NapCat临时文件且吞错误，不能当按流取消或清理成功证明。
受检HTTP新增逐帧消费，保留原socket/进程绑定、令牌后置、身份与总截止；增量有界JSON解析保留完整OneBot对象，单记录沿responseBytes、总线192MiB、4096记录硬预算，EOF半包/超限/未知形状明确失败，无重试。仅显式api stream ACTION --params-file/stdin --request-id允许动态JSON包；未知action仍可尝试且按写/有键管理，目录不是名单。已知流action的api call明确要求api stream，避免把多包HTTP当普通JSON后丢失效果。
api stream复用原参数解析/目标效应、mediaDownload/read/send及共享api.action意图和HMAC；起流前pending/身份，返回ready、provisional packet和最终summary的NDJSON。流中原生数据只是暂存观察，最终身份/EOF与状态确认后summary才可确认；写入断连/确认前输出失败/无最终确认均unknown，不自动重新连接或发送；缓存只summary且不重取包，原生正文/字节/路径/凭据不入ledger。上游upload分块确认只标action接受，transferComplete须明确file_complete；原始包保真不放宽其它领域DTO。
media download MEDIA_REF --output FILE --stream沿f1重新核对来源，file/record才使用相应native stream；image继续既有已绑定HTTPS流式读取，不用全局名称搜索替代来源证明。逐块核对类型、索引、base64/大小与总计，转换后不使用上游错误沿用的原音频file_size。先有界临时落盘，最终身份/完成校验后沿既有无覆盖发布，不写部分目标文件、不自动播放/跟随URL。未知形状/结束不完整不发布。
file stage FILE --request-id KEY以已有128MiB/缓存配额/受管不可变影本进行后端缓存分块上传，是显式底层文件准备任务，不发送聊天消息。每流ID/文件名由当前账号运行及意图的带密钥摘要生成安全固定名，禁止调用方路径/覆盖其它缓存；单总意图覆盖创建、64KiB块、完整提交，认证源影本保留后再请求，任何中断不重放。必须核对所有确认计数及最终大小/SHA256；仅首次成功返回nativePath数据和请求保留期300秒，缓存不恢复/读取文件。该路径不授予本机读取/删除权限，也不冒充f1/文件列表别名；后续业务仍须显式授权。
六action派生实际参数Schema；下载/测试为read（下载仍需mediaDownload），上传/清理为write，get/private/file类裸来源不能用额外group_id证明受限目标。test_download_stream仅作显式兼容测试工具；流式临时全清理只保留直接入口，不自动执行。真实大文件/音频转换/网络中断、上游内存/文件生命周期和macOS另验。

## 闪传与目录影本（R12）

固定file/flash八action、core/apis/flash.ts、data/flash.ts及FlashTransfer服务。flash create PATH... [--name TEXT] [--validity-days 14|90|180] --request-id KEY支持1至32个普通文件/目录根；原生validitySeconds显式映射1209600/7776000/15552000，不能省略（源注释记录新内核缺失可崩溃）。源实际返回GeneralCallResult/createFlashTransferResult的fileSetId/shareLink/expireTime，不使用示例task_id；返回只证明任务创建，异步上传完成和源文件复制时机无证明，全部影本认证persistent，普通prune保留并计配额。缩略图沿上游单媒体自动生成/多根默认限制，裸thumb_path参数如实保留，不伪造已生成封面。
目录快照只用于明确支持目录的已审阅action，复用受管attachments根/签名运行前缀；拒绝链接/特殊文件，限深16、256子项、单根总128MiB且含非空内容。复制文件/空子目录，内容/相对路径/目录类型清单排序哈希用于稳定身份，复制后复核源和影本；持久pending/实际身份后才签名保留。私有指纹区分目录与普通文件，旧文件指纹/认证字节保持；认证sourceDirectory标志及persistent共同约束来源验证，不因配额用的目录哨兵授予文件所有权。递归元数据配额扫描有界，不读取未知内容或删除现场；旧平面缓存规则保持，目录恢复重核全树内容摘要。
flash info FILESET投影文件集与原生上传/下载计数/状态，flash list FILESET --limit N投影固定首批（上游count18、reqDepth1、无公开分页参数），complete:false，不自动遍历本地/远端目录或制造f1引用；省略本地路径/设备等字段。原生result非零拒绝。flash resolve CODE只交原生分享码解析（源对链接取第一个等号后字符串，不是CLI访问URL）；原处理器丢错误码，只接受非空fileset_id，不以空ID当成功。
flash link FILESET返回shareLink/原生过期值，flash url FILESET --name NAME|--index N取transferUrl；URL只是显式结果数据，均不自动访问。index为上游返回批次内从0起的首个匹配，name与index互斥，不能把cliFileIndex或多个批次当全局位置。
flash send FILESET CONVERSATION --request-id KEY发送明确私聊/群卡片；裸group_id非空优先于user_id，受限targets时拒绝含糊类型转换。原生errCode及rsp.sendStatus逐项核对，不伪造message_id或m1；无明确状态为unknown。flash download FILESET --request-id KEY仅启动SDK默认下载目录任务，需send/read/mediaDownload，不提供任意后端路径或本机输出完成承诺。
已有文件集读取/链接/下载不能证明聊天来源，targets受限时拒绝；显式本地创建为自身资产，发送只使用实际接收目标。所有写任务沿共享api.action意图、HMAC与一次调用，create用绑定内容路径计划。首次结果有限投影，不持久原始结果/链接/正文；缓存不重读。下载/创建接受与传输完成分别说明；真实文件/目录、原生状态、缩略图、默认目录及macOS仍待验收。
