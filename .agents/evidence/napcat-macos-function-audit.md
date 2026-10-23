# NapCat Windows 与 Apple Silicon 功能审计

文档编号：CHECK-069。状态：静态审计完成，完整运行兼容性未验证。日期：2026-10-23。
关联：FACT-005、SPEC-004、RFC-0008、TASK-006。仅以 Apple Silicon 原生 arm64 为新增产品目标；Intel/Rosetta 不在支持承诺内。

## 基线与方法

固定 NapCatQQ v4.18.38 / `bbd0d2aca3d470c20b33b1d4f95123335633bf26`；发布包、安装器及文档基线沿用 CHECK-068。源码归档只放忽略研究目录，抽取的 851 个文本文件逐个按 Git blob 算法与固定 Git tree 比对，全部一致。未执行上游脚本、原生模块或 QQ，未读取私人账号材料。

附属 `napcat-macos-actions.csv` 是原创静态索引：以 router.ts 中 ActionName 的符号/字符串声明为范围，再匹配 action 目录中 `actionName = ActionName.<符号>` 的赋值位置。共 182 项声明，180 项找到赋值；`.get_word_slices` 与 `unknown` 未找到。包括测试接口，不等于 182 个已注册/可运行业务 API。清单不解析动态注册、继承后的完整调用图或原生闭源实现，所有命中的接口仍为 runtime-unverified。WebUI、事件、插件及生命周期另列下表；本轮不是上游全部功能逐项运行验收。

## 功能矩阵

| 功能 | 固定源码/资产观察 | Apple Silicon 结论及 QQ CLI 影响 |
| --- | --- | --- |
| Framework 加载、官方窗口、共享会话 | 有 Darwin wrapper 路径和共享 session 结构；包内原生启动资产仍为 Windows EXE/DLL/BAT | 尚无满足项目约束的原版 macOS 外部加载路径，仍是首要前置缺口 |
| Shell 安装及登录 | 官方 Mac 安装器选择 Shell；与原 app_launcher 互斥，启动前按名称强杀 QQ | 有部署路径不等于满足本项目官方窗口共用会话、多实例及不抢焦点要求 |
| HTTP、WS、反向 WS、鉴权与事件 | OneBot 共用 JS 实现；事件依赖 QQ session 回调及消息转换 | 协议层可复用；监听、通知、断线恢复、账号变化隔离均未在 Mac 运行验收 |
| 状态、身份、好友、群、成员、最近会话 | 有共同 action 实现，底层依赖各 QQ 内核服务 | 未发现即可判定这些接口 Windows 专属的直接分支；仍不能证明 Mac 服务 ABI、参数及缓存行为一致 |
| 普通私聊/群聊、回复/提及、消息读取、历史 | 共用实现，依赖 QQ 消息服务、消息映射和转换 | 需要固定 QQ 版本实测；Windows 的消息 ID、分页/历史覆盖证据不能转用 |
| 合并转发 | GetForwardMsg 经 MsgApi 解析；multiForwardMsgElement 在正常读取无结果时尝试 Packet FetchForwardMsg | 即使 action 本身没有平台分支，也可能间接依赖版本偏移及 hook；不可归为纯 HTTP 兼容 |
| 图片、语音、视频、文件、群文件、流传输 | Darwin arm64 FFmpeg 模块确实随包发布；上传/下载仍依赖 QQ 富媒体服务及转换 | 文件路径、下载内容、编码、大小/中文名、临时文件和事件都需验证；登录成功不是媒体验收 |
| FFmpeg 自动部署与回退 | download-ffmpeg.ts 的自动下载/安装路径限定 win32；适配器可显式选外部 ffmpeg/ffprobe，macOS 搜索含 Homebrew 目录 | 有平台差异；默认选中的原生 addon 加载失败会抛错，不自动切换 exec。不能声称媒体全不可用或已兼容 |
| Packet 扩展、RKey、转发/文件 URL 等 | napi2native.json 有 23 项 Mac arm64 版本键，packet.json 有 34 项；两表用途不同；有 darwin.arm64 原生加载分支 | 版本键不是兼容白名单；需 QQ 完整版本、系统、架构、模块加载、send/receive hook 和具体操作验证 |
| OCR：ocr_image / .ocr_image | actionDescription 明确“仅Windows端支持”；SystemApi 调用 getNodeMiscService().wantWinScreenOCR | 无已证实的 Mac 对等实现；不把描述说成已有运行 OS guard。QQ CLI 当前 18 个 Action 不包含 OCR |
| WebUI 终端 | Log.ts 在 darwin 上直接返回“MacOS不支持终端”；发布 bundle 同样拒绝，PTY 资产无 Darwin | 明确不具备 Windows 全功能对等；CLI 不依赖上游 WebUI 终端 |
| 设备 GUID 与 DPAPI | Registry20 加解密仅 win32；QQLogin 有独立 MacMachineInfoUtils 路径 | 属于不同实现，不能移用 Windows 注册表/DPAPI 流程；未读写真实设备标识 |
| 重启、退出与掉线恢复 | Data.ts 默认重启回调返回未初始化；源码中 Shell napcat.ts 注册回调 | 受 Shell/Framework 模式影响，不是仅按 OS 判断；不能用 Shell 管理接口证明 Framework 生命周期可用 |
| WebUI 配置/更新、插件及其它扩展 | 有共用实现不代表外部命令、插件依赖、文件权限及更新入口可移植 | 未逐插件或逐管理操作验证，不纳入“全部兼容”结论；插件兼容还依赖具体插件版本 |
| can_send_image / can_send_record | 共用 CanSend._handle 固定返回 yes:true | 不能作为平台媒体能力探针或验收证据 |

## 本项目必要 API 与验证差距

当前 transport/client.rs 的 18 个 Action 均在上述源码声明/实现赋值索引内：

- 身份/状态：get_status、get_login_info、get_version_info。
- 目录/会话：get_friend_list、get_group_list、get_group_info、get_group_member_list、get_recent_contact。
- 消息/历史：get_msg、get_friend_msg_history、get_group_msg_history、get_forward_msg。
- 写入：send_private_msg、send_group_msg、upload_private_file、upload_group_file。
- 媒体读取：get_image、get_file。

这 18 项仍全部缺 Mac 真机证据；另需验证登录前观察/控制、鉴权 HTTP/WS、通知与生命周期。终端/OCR 差异本身不阻塞当前 CLI API，但不能据此断言其余接口均兼容。API 存在、路由注册、原生方法可调用、结果满足 CLI 契约是四个不同层级。

## 固定源码定位

下列路径均相对固定 [源码根](https://github.com/NapNeko/NapCatQQ/tree/bbd0d2aca3d470c20b33b1d4f95123335633bf26)，不是可变 main 来源：

- [接口声明](https://github.com/NapNeko/NapCatQQ/blob/bbd0d2aca3d470c20b33b1d4f95123335633bf26/packages/napcat-onebot/action/router.ts)；action/index.ts 与 action/auto-register.ts 负责注册。
- [终端限制](https://github.com/NapNeko/NapCatQQ/blob/bbd0d2aca3d470c20b33b1d4f95123335633bf26/packages/napcat-webui-backend/src/api/Log.ts)。
- [OCR 声明](https://github.com/NapNeko/NapCatQQ/blob/bbd0d2aca3d470c20b33b1d4f95123335633bf26/packages/napcat-onebot/action/extends/OCRImage.ts)；packages/napcat-core/apis/system.ts 的 ocrImage。
- [FFmpeg 选择和失败语义](https://github.com/NapNeko/NapCatQQ/blob/bbd0d2aca3d470c20b33b1d4f95123335633bf26/packages/napcat-core/helper/ffmpeg/ffmpeg-adapter-factory.ts)；同目录 download-ffmpeg.ts、ffmpeg.ts。
- [转发回退](https://github.com/NapNeko/NapCatQQ/blob/bbd0d2aca3d470c20b33b1d4f95123335633bf26/packages/napcat-onebot/api/msg.ts) 的 multiForwardMsgElement；action/go-cqhttp/GetForwardMsg.ts。
- packages/napcat-core/external/napi2native.json、packet.json；packet/handler/client.ts、napi2nativeLoader.ts。
- packages/napcat-webui-backend/src/utils/guid.ts、api/QQLogin.ts、helper/Data.ts；packages/napcat-shell/napcat.ts。
- packages/napcat-onebot/action/system/CanSendRecord.ts、CanSendImage.ts。

## 开发判断

“NapCat 的 Windows 所有功能在 Mac 完全对等”已被终端限制反例推翻；不能承诺所有开发顺利。可继续独立的平台隔离、原创合成验证，但完整本地接入尚未满足开发前置证据。

下一步须先确定符合既有约束的原版 arm64 Framework 加载路径，再固定 macOS/QQ 完整版本元组。CHECK-068 中 napi2native/MoeHoo 声明 minos 15.0，官方指南 12+ 不足以确定产品系统下限。之后在明确授权的实验范围内验证窗口/共享身份及上述 CLI 必需能力；真实操作授权未因本轮静态审计扩大。若原版入口不可得，先记录具体产品取舍，不自行写加载器或改用 Shell/LiteLoader。
