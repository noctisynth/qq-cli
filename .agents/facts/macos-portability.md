# Apple Silicon macOS 接入事实

文档编号：FACT-005。状态：固定源码/发布包观察及限定功能真机验证。更新：2026-10-27。
详细证据：CHECK-068～072（调研/原型）、CHECK-133～136（正式CLI/数据/Keychain/功能）。需求：RFC-0008；技术：SPEC-008；待办：TASK-006。

## 当前结论与适用范围

| 事实 | 依据与边界 |
|---|---|
| 存在保留官方 QQ GUI、共享真实会话的 arm64 接入路线 | CHECK-072；专用官网 QQ 7.0.2-53644 副本、本地重签名、薄适配、原版 Framework v4.18.41、macOS 27.0 |
| 正式 Rust CLI 的固定组合正常功能及本地安装升级已验收 | CHECK-133～136；原生arm64、macOS27.0、官网QQ7.0.2-53644、Framework v4.18.41/v4.18.38，不推导其它平台/故障矩阵或公开发布 |
| 原版 napcat.cjs 不能直接用于该 Mac QQ | 当前会话类没有其假设的 create；nativeLoader.cjs 可接收已有 session/loginService |
| 会话就绪可从官方真实初始化观察 | onOpentelemetryInit 严格 true 与匹配代次；账号散列与 OneBot 身份一致，不靠假通知 |
| 正式CLI读取、授权发送、附件、限定事件、重启及切版/回滚通过 | CHECK-136；原型CHECK-072仅有10个限定读取接口，其跳过项不作为当前未实现清单；详细范围与差距见TASK-006 |
| 原 NapCat 文件保持一致 | 680 个 Framework 文件与固定官方 ZIP 字节匹配；不是修改后的 NapCat fork |
| 副本原生 hook 有签名权益要求 | 该组合需要主进程 unsigned executable memory 等权益；不等于其它版本同样可用 |
| 路径和配置不是仅靠 NAPCAT_WORKDIR 即可隔离 | QQ 还计算容器共享路径；需受控用户目录，OneBot 配置名为 onebot11.json |
| Framework默认配置目录为NAPCAT_WORKDIR/config | 固定v4.18.41源码NapCatPathWrapper；config/logs/cache/plugins均由writePath派生，static仍位于binaryPath；CHECK-117 |

原型基线为 v4.18.41 / 859eb530c5801577633c2c3ae871d23586edf281，固定摘要、方法和失败记录在CHECK-072。正式CLI另以该版及v4.18.38 / bbd0d2aca3d470c20b33b1d4f95123335633bf26完成CHECK-136的切换/回滚链，两版证据独立。早期v4.18.38的静态结果保留在CHECK-068/069，不混写为同一次实验。
原安装未修改、系统 Gatekeeper/SIP 未关闭；专用副本的 ad-hoc 身份不同于腾讯签名。
上游有界检索未确认可复用的保留完整原签名入口，不是对所有可能方案的不存在证明。

## Shell 与 Framework

固定 [Mac 安装器](https://github.com/NapNeko/NapCat-Mac-Installer/blob/accafd630b55656035a24486f56b3c08cb0505d1/NapCatInstaller/Utils.swift)
选择 Shell 入口或官方 app_launcher，正常 Shell 路径不创建官方 QQ 聊天 GUI；这是控制流/发布代码观察，不是 Shell 视觉运行验收。
Shell 自行初始化会话；其进程管理、Darwin 路径和原生加载可参考，不能据此证明与 GUI 共用会话。
上游安装器按名称终止 QQ/QQEXDOC 的流程不满足本项目精确实例所有权要求。
Framework 的共享对象结构与原版 nativeLoader 是当前适配基础，外部入口和真实时序仍需固定组合验证。

## 已确认差异与未知

- WebUI 终端在 darwin 明确拒绝；OCR action 描述为 Windows 专属。当前 CLI 必需接口不含这两项。
- Registry20 DPAPI 加解密限定 Windows；macOS 使用不同机器信息路径，不能移植 Windows 凭据机制代替。
- Darwin arm64 Packet/Napi/FFmpeg 资产存在且原型可加载；不能因此声称媒体或全部 Packet 扩展都通过。
- CHECK-068 中部分原生模块 minos 15.0，与指南 macOS 12+ 有差异；单台 27.0 实测不能确定产品最低系统版本。
- Intel/Rosetta 不在需求范围；其它QQ/系统/NapCat组合、多实例和长期稳定性需独立证据；固定两版的正常更新/回滚已有CHECK-136，不等于完整维护故障矩阵通过。
- 正式CLI使用libproc/kqueue、audit token信号、Keychain及同socket归属核验，源码与合成/真实证据见SPEC-008及其CHECK；单次监听检查仍不等于运行所有权或PID重用防护。

## 本地适配事实

协议/大部分领域逻辑可复用；Windows Job、FILETIME、凭据、QQ发现及生命周期按平台隔离；受检HTTP/WS复用协议实现并选择平台归属守卫。
CHECK-067 已完成原生目录隔离和非 Windows 类型检查，不等于 macOS 启动支持。
技术边界在 SPEC-006/008，实施差距由 TASK-006 维护；FACT 不保存执行顺序或任务计数。

## 默认工作目录配置路径

2026-10-24核对固定提交859eb530c5801577633c2c3ae871d23586edf281的[packages/napcat-common/src/path.ts](https://github.com/NapNeko/NapCatQQ/blob/859eb530c5801577633c2c3ae871d23586edf281/packages/napcat-common/src/path.ts)：NAPCAT_WORKDIR存在时用作writePath；未提供时macOS回退用户Library/Application Support/QQ/NapCat。配置、日志、缓存及插件目录从writePath派生；静态资源从binaryPath派生。此为固定源码观察；正式CLI配置目的地采用该路径关系，运行验证边界见CHECK-136，原型边界仍见CHECK-072。

## 磁盘释放的身份边界

2026-10-24核对Apple公开源码：DiskArbitration-535.0.10固定提交a542bda934211dc3c301bfdcc7f21349c4164a85的[DADisk.c](https://github.com/apple-oss-distributions/DiskArbitration/blob/a542bda934211dc3c301bfdcc7f21349c4164a85/DiskArbitration/DADisk.c)中DADiskCreateFromIOMedia读取BSD Name并转交DADiskCreateFromBSDName；[DiskArbitration.c](https://github.com/apple-oss-distributions/DiskArbitration/blob/a542bda934211dc3c301bfdcc7f21349c4164a85/DiskArbitration/DiskArbitration.c)传递字符串ID，[DAServer.c](https://github.com/apple-oss-distributions/DiskArbitration/blob/a542bda934211dc3c301bfdcc7f21349c4164a85/diskarbitrationd/DAServer.c)按该ID查找磁盘。仅保存DADiskRef不足以推断原IOMedia代次的原子效果绑定。

IOStorageFamily-337.100.1固定提交7edb88fbae296fb7c8ce2f64e115e116e566d51c的[IOMediaBSDClient.cpp](https://github.com/apple-oss-distributions/IOStorageFamily/blob/7edb88fbae296fb7c8ce2f64e115e116e566d51c/IOMediaBSDClient.cpp)在terminate时将仍有open的minor标为obsolete并延迟到close删除；MinorTable::update仅复用特殊orphaned槽位，其说明涉及DKIOCFORMAT。DKIOCEJECT要求同anchor仅有发起者这一个open并成功独占打开驱动，否则返回EBUSY；尝试后进入open-flux，故不得重用该FD重复发起。项目不使用format、force或替代路径。该公开源码版本不等于已证明本机内核与其逐字相同；SDK27.0定义和本机合成行为分别验证，见CHECK-125。

## v4.18.38 发布包复核

2026-10-25从[固定官方Release资产](https://github.com/NapNeko/NapCatQQ/releases/download/v4.18.38/NapCat.Framework.zip)重新取得原版ZIP，核对既有目录提交bbd0d2aca3d470c20b33b1d4f95123335633bf26的资产标识：36,728,573字节，SHA-256为45ef7f46aa78588c636044b6226b4b1847e31287ab6ec36b2d48adaa9785097b。ZIP有806个条目，其中680个普通文件；目录条目不计入文件数。package.json摘要为a4a29f9ecd82642d73d6f778a06e3a72531a2e52fc925fc0f46ed23519976ceb。

三个darwin.arm64节点的头部CPU类型为arm64，摘要与v4.18.41固定成员逐字节相同：

| 成员 | SHA-256 |
|---|---|
| native/napi2native/napi2native.darwin.arm64.node | 5a48995703afbb2889c73ee4624b1df41dc5fbc563e77d9224aa5f2aec048953 |
| native/packet/MoeHoo.darwin.arm64.node | 72e9c8ca9ba48f5e3e8ffd5182d57e3f567dcd71331b04fd3aacdf1c4b36e7bf |
| native/ffmpeg/ffmpegAddon.darwin.arm64.node | c578932c04b40e36b32601a86d3833dbd9892df13bd13dbe9355623da0144cbf |

nativeLoader.cjs和napcat.cjs摘要也与v4.18.41相同；napcat.mjs不同，其v4.18.38摘要为f79260cb7e6c4b579a4346ffd39a91a33ea73f9b7cd44d9ce204ab471096601a。此为发布包静态身份观察，单独不证明v4.18.38的GUI共享、事件、媒体或CLI运行兼容，也不能从相同原生模块继承v4.18.41原型结论。后续正式CLI的v4.18.38验证另见CHECK-136；两版仍使用各自完整软件/系统键和独立在线证据。
