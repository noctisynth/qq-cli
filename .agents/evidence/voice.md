# 语音获取与显式转文字

文档编号：CHECK-148。状态：R18离线通过。日期：2026-10-29。
关联：RFC-0009、SPEC-009、TASK-007、CHECK-147。

固定NapCat v4.18.38/bbd0d2aca3d470c20b33b1d4f95123335633bf26的file/GetRecord.ts、file/GetFile.ts、msg/FetchPttText.ts、api/msg.ts及common/src/file-uuid.ts已核对。record段以消息元素缓存ID绑定f1；下载前核对m1原消息/账号/实例/运行，不跟随后端path/url。get_record按读取归类但保留mediaDownload及目标权限，原格式out_format空串；八种显式转换需上游转换器。转换源保留旧file_size，实际保存字节独立计数和摘要，受128MiB预算及原子不覆盖发布限制。缺base64仍明确不可取，不代读后端路径。

message transcribe M1显式触发translatePtt2Text，因此为有键写任务，要求read/send及来源目标许可。首次意图内核对消息含record，只发一次业务调用；仅非空text为accepted，首次输出严格文本DTO。上游通用异常无法区分任务未启动/进行中/失败/无文本，失败或空文本为unknown；缓存和request get不再次识别、不保存文本。动态JSON仍仅API边界，领域适配用实际类型反序列化，未新增全局Value桥。语音发送、自动播放/批量识别未纳入。

原创受管CLI夹具覆盖原格式群语音、MP3私聊转换（原size=3、实际9字节）、无格式转换的群/私聊文件回归、非record转换拒绝和安全不覆盖。转文字八分支覆盖成功/空文本/原生拒绝、非语音、过期run、来源目标/read/send拒绝；成功预览离线，重放不联网，查询无正文，并扫描合成状态证明文本/原生扩展未落盘。实际输出匹配派生Schema。

首轮发现遗漏格式类型Schema注册及受管媒体Record分派，修复后重跑通过。CLI14、API3、Skill3、转文字2、媒体CLI1及消息/媒体/来源/转发/撤回/公共契约/输入/截止适用回归通过；all-targets Clippy、结构/契约、Skill quick_validate、源码包白名单472文件、fmt/diff通过。Semifold仅检查11份待发布变更集计划，未消费或发布。

未执行真实QQ或macOS语音验收，未证明任何实际编码器、QQ识别效果或真实端到端资源可用；整体功能及其它组仍按TASK-007继续。
