# NapCat 架构研究基线

文档编号：FACT-001。状态：Source-observed。更新：2026-10-18。
来源：EVIDENCE-001，承接建项前本次对话中的公开源码阅读。
后续固定版本及真实附件核对见 FACT-002、DATA-001、CHECK-002；本页早期源码调查不构成运行验证。

## 已观察内容

2026-10-18固定增量见CHECK-015：v4.18.38固定提交与匹配摘要发布包均显示Framework先调用InitWebUi，再等待登录，随后创建协议适配器；WebUI与OneBot HTTP的所检构造链直接在Node内建监听，支持同进程候选的架构推断，但未实测QQ PID。WebUI配置端口忙时可递增，CLI不能跟随扫描或从日志取带token地址。该增量不替代真实兼容/生命周期验收，RFC-0003候选仍未冻结。

- Windows Shell 的 launcher.bat 定位已安装 QQ，传递程序路径与 NapCatWinBootHook.dll 给启动器。
- 公开 Windows Boot 源码创建暂停的新进程，加载 DLL 后恢复运行；不是现有 PID 的接管接口。
- Shell 的 qqnt.json 将入口指向 loadNapCat.js；加载器导入 napcat.mjs。
- Shell 自行加载 wrapper.node 并初始化引擎、登录服务和会话；复用安装文件不等于共享现有会话。
- Framework 的 napcat.cjs 在原生模块加载处接入，并通过代理向 QQ 返回共享会话和登录服务。
- nativeLoader.cjs 接收 session、loginService 和初始化回调并调用 NCoreInitFramework；
  此函数签名没有 LiteLoader 参数。这不足以证明发布包支持独立加载或与当前 QQ 兼容。
- 消息 API 通过内部消息服务执行，不依赖输入框点击，因此具备避开桌面输入竞争的架构基础。

## 推断与未知

独立启动加载器安装接入逻辑、继续官方入口、共享会话，再启动 API，在架构上有依据，尚无原型。
不能仅把 Shell 入口和官方入口同时运行就认为共享会话；重复初始化可能冲突。
不能保证双进程同账号共存，也不能保证加载器与发布产物接口、初始化时序或新 QQ 兼容。
不依赖 LiteLoader 不意味着不注入；源码中存在 Hook 和原生处理逻辑。
风险概率、协议合规性、封号情况、性能和全历史完整性均未做验证，不提供肯定结论。

本页早期 main 来源仅作背景，本轮 Framework 审计以 FACT-002 固定基线为准。
Shell 与另一 Windows Boot 仓库未在本轮和发布包完整匹配，不把其源码套用到 Framework 原生加载器。
重要修正：实际 Framework 包已有不经 LiteLoader 脚本的原生入口，自有加载器不是已确定前提。

## 固定产物消息与附件观察

检索日期2026-10-18；基线为 FACT-002 登记的 v4.18.38 完整 Framework 产物，napcat.mjs
SHA-256 f79260cb7e6c4b579a4346ffd39a91a33ea73f9b7cd44d9ce204ab471096601a，未运行真实会话。

- SendMsgBase.base_handle 将普通消息段交给 createSendElements，再调用 sendMsgWithOb11UniqueId，最终返回 message_id。
- createValidSendFileElement/createValidSendPicElement 内部调用 FileApi.uploadFile；转换与最终发送属于同一次上游 action 的内部流程，CLI不能据最终应答推导每个内部阶段的独立完成时间。
- GoCQHTTPUploadPrivateFile/GoCQHTTPUploadGroupFile 的 _handle 都构造文件消息并调用 sendMsgWithOb11UniqueId，返回 file_id 或 null。名称中的 upload 不代表无聊天副作用的纯上传，不可当作发送前可重复准备。
- 以上为固定发布 JavaScript 的静态观察，不证明私聊/群文件的真实成功率、混合段兼容或送达；CLI 不复制上游实现。

实施含义：附件整体写意图必须早于这些 action；不能在 upload_* 成功后无条件再 send_*。
文件传输和聊天接受状态只报告可观察证据；不暴露的内部阶段保留 unknown，不把 null file_id 解释为无副作用失败。

事件静态补充（同一固定v4.18.38 bundle与上述SHA-256，2026-10-18）：OB11HeartbeatEvent为meta_event/heartbeat，状态含online/good；OnMsg事件分发按适配器reportSelfMessage决定是否上报自发消息，默认配置字段为false，自发或私聊补target_id=peerUin。parseMessageV2根据自身senderUin设置message_sent_type=self。只是静态字段和条件证据，不证明WS配置已启用或实际事件无丢失；CLI应按实际发送者/target_id定位自发私聊，不以自发上报默认存在作假设。

通知静态补充（同一固定bundle、2026-10-18）：OB11FriendRecallNoticeEvent含user_id/message_id；OB11GroupRecallNoticeEvent额外group_id/operator_id；OB11FriendAddNoticeEvent含user_id。OB11GroupIncreaseEvent/DecreaseEvent含group_id/user_id/operator_id/sub_type，调用处对无法解析的成员或操作者回退0，增加子类型invite/approve；OB11GroupAdminNoticeEvent为set/unset。这些是投影字段证据，不是CLI真实通知回归。

附件静态补充（同一固定bundle、2026-10-18）：消息段image/file分别经handleOb11FileLikeMessage及createValidSendPicElement/createValidSendFileElement。图片转换在uploadFile后调用imageSizeFallBack，空文件拒绝；图片返回path加入deleteAfterSentFiles。文件转换同样拒绝空文件，uploadGroupFile时加入清理列表。CLI必须传自有副本，不能据这些内部步骤断言可独立重试或真实混合段兼容；普通file段与upload_*均不能重复组合发送。

媒体来源边界补充（固定v4.18.38 bundle、同前SHA-256，2026-10-18）：NapCatPathWrapper.cachePath位于NAPCAT_WORKDIR/cache；NapCatCore.dataPath读取NodeQQNTWrapperUtil.getNTUserDataInfoConfig，NapCatDataPath为该目录/NapCat，NapCatTempPath再加temp，fallback为用户QQ配置路径，不由NAPCAT_WORKDIR等同控制。downloadMedia返回onRichMediaDownloadComplete.filePath，downloadFileById返回onFileStatusChanged.filePath。GetFileBase/GetImage在enableLocalFile2Url=true时从完成路径读base64返回，默认该配置为false；payload支持file/file_id。以上静态依据说明不能把NapCat工作缓存视为所有QQ下载授权根；base64字节方案仍需配置与有界传输验证，不证明真实媒体可取。

媒体句柄补充（同一固定产物及SHA-256，2026-10-18）：FileUUIDManager.encode以customUUID优先作TimeBasedCache键，put覆盖同键；FileNapCatOneBotUUIDWrap默认TTL为864e5毫秒。图片转换以element.fileName注册并仅输出file别名，普通文件另注册element.fileUuid并输出file_id；GetFileBase先decode取得peer/msgId/elementId再下载。uploadFile保留输入basename（必要时加扩展名），因此不能推断图片名称一定为内容摘要或全局唯一。架构推断：同名消息并发覆盖可使文件名下载定位偏离原消息，get_msg刷新本身不能排除窗口；此为静态来源风险，未在真实QQ会话复现，不等于所有file_uuid均具有不可碰撞证明。

图片URL补充（同一固定产物及SHA-256，2026-10-18）：IMAGE_HTTP_HOST为https://gchat.qpic.cn，IMAGE_HTTP_HOST_NT为https://multimedia.nt.qq.com.cn。getImageUrl从element.originImageUrl解析appid/fileid，appid为1406/1407且有fileid时构造/download?appid=…&fileid=…&rkey=…，旧主机额外spec=0；否则用md5HexStr构造/gchatpic_new/0/0-0-{MD5}/0。marketFaceElement以emojiId前两字符为dir，构造https://gxh.vip.qq.com/club/item/parcel/item/{dir}/{emojiId}/raw300.gif。GetMsg先按已解析peer/msgId调用getMsgsByMsgId，再parseMessage生成段，未经过文件名缓存下载定位。以上是静态来源构造证据，不证明URL仍有效、域名DNS/TLS或真实下载成功；rkey获取存在原版其他网络逻辑，不将其解释为CLI可任意请求第三方rkey服务。

预登录配置静态补充（同一固定v4.18.38产物与SHA-256，2026-10-18，重新核对摘要）：NapCatPathWrapper.configPath为NAPCAT_WORKDIR/config。ConfigBase.read优先name_<selfInfo.uin>.json，缺失时读取name.json再save账号文件；OB11ConfigLoader使用onebot11，NapCatConfigLoader使用napcat。Framework在等待登录之前InitWebUi，WebUI读取webui.json；OneBot适配初始化使用登录后的selfInfo。

配置字段观察：HTTP支持host/port/token/enableCors/enableWebsocket/messagePostFormat/debug；正向WS另有reportSelfMessage/enableForcePushEvent/heartInterval。OneBot根含enableLocalFile2Url、parseMultMsg、imageDownloadProxy及timeout，反向HTTP/WS、SSE、plugins是独立数组。WebUI默认host为双栈通配、端口6099，支持独立token、autoLoginAccount、IP访问控制及代理头开关。InitPort冲突时可递增端口，环境优选端口失败仍回退；listen回调打印token及带token查询的URL。NAPCAT_WEBUI_SECRET_KEY覆盖分支同样打印秘密；JWT secret默认取环境或Math.random派生值。此为静态配置/日志行为证据，未运行原版或接触实际配置；不能据模板字段推断真实账号隔离、监听归属、无日志泄漏或兼容已成立。
