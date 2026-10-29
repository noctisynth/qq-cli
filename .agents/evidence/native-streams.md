# 原生流协议与受检分块文件链路

文档编号：CHECK-158。状态：R11六action协议、裸流与文件链路离线通过。日期：2026-10-30。
关联：RFC-0009、SPEC-009、TASK-007、CHECK-157、CHECK-148。

固定NapCat v4.18.38/bbd0d2aca3d470c20b33b1d4f95123335633bf26的stream六action、BaseDownloadStream/StreamBasic、OneBotAction.ts和network/http-server.ts已核对。HTTP每包为OneBot对象加CRLF空行，最后另写handle结果；上传每次RPC只是创建/一块/提交/查询，收到chunk_received不是文件完成。源reset清理后抛错，clean_stream_temp_file全清临时目录且吞错误，不当按流取消或清理成功证明。六实际载荷派生Schema及responseMode目录完成；已知流的api call在效果前提示api stream，不作为普通JSON误派发。

受检HTTP共享逐帧实现，保留原socket/进程绑定、凭据后置、响应后归属与总截止，并在帧交付前重核归属；无普通HTTP降级或自动重试。原单JSON post复用有界聚合适配。增量解析先借用RawValue核对单记录预算再建JSON值，保留完整OneBot包；总线192MiB、4096记录、单条responseBytes。两单测覆盖逐字节/UTF-8/转义/空行/EOF、半包、非对象、单包超限和记录数上限。

api stream允许未知未来action，原参数解析/效应/权限/目标与共享api.action键和HMAC保持；ready后返回provisional packet，最终身份/协议确认的summary才接受，异步/缺结束/输出或连接中断未确认均unknown。上传单RPC正确stream_id/类型确认可接受，transferComplete仍false，只有file_complete才标完整；不保存原始包/参数/秘密，缓存只有摘要，跨api call入口同键也不重发。输出工作进程保持原有单行/截止/取消语义，最终结果落盘后摘要输出失败不会再产生业务请求。

file stage用一个总意图覆盖影本保留、创建、64KiB逐块、显式完成；名称/流ID取请求/运行/账号/内容的带密钥摘要，非调用方路径。逐个确认类型/计数、最终大小/SHA256及身份；源文件沿原配额/128MiB上限。请求300秒后端保留；首次返回nativePath数据，缓存pathAvailable:false，不恢复/读取原文件。中断不自动reset/续传/另起请求，路径不授予本机读取或删除权限。

media download --stream仍先f1重读原消息并检查read/mediaDownload/源目标；file/record用原生块，image保留既有身份绑定HTTPS流。检查info/chunk/complete顺序、索引、base64/大小/总计与源长度，转换音频不信任上游沿用的原file_size。内容先匿名暂存，最终身份/完成确认后复用无覆盖发布；失败不发布目标文件，不自动播放/跟随返回URL或后端路径。

原创受管CLI22分支通过：未知裸流成功/缺完成/原生失败/异步/半包/结束后多包/超限/切号、上传确认和错流ID；stage正常/中断/错哈希/切号/权限/前置身份拒绝及缓存；文件/音频转换下载、错索引/缺完成/切号/media拒绝。每行实际结果匹配派生Schema；缓存及跨JSON入口零额外Native调用，ledger无正文、参数、块数据或后端路径；下载失败无目标文件。匿名暂存与源影本均合成数据，未调用真实QQ。

回归通过：API单测13、codec2、streams2、CLI14、Skill3、API3/API CLI1、deadlines2、files2、media_cli1、owned_transports8（IPv4/IPv6、固定/分块/媒体、归属/认证/超时/协议、控制与WS）。all-targets Clippy、fmt、架构/契约、Skill quick_validate、源码包白名单513文件、diff通过。128叶命令/146帮助入口，11任务/14导出文件；Semifold仅规划21份变更集至0.2.0，未消费/发布。

未重跑完整suite/release，原完整证据按CHECK-152快照。未验证真实大文件、SDK缓存生命周期/取消、音频编码、macOS新链路及实际输出阻塞故障全矩阵。临时全清理仅明确直接入口，未自动执行；R16/R17仍暂停。
