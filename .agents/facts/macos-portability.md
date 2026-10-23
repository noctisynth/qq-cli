# Apple Silicon macOS 接入事实

文档编号：FACT-005。状态：固定源码/发布包观察及有限真机验证。更新：2026-10-24。
详细证据：CHECK-068/069/070/071/072。需求：RFC-0008；技术：SPEC-008；待办：TASK-006。

## 当前结论与适用范围

| 事实 | 依据与边界 |
|---|---|
| 存在保留官方 QQ GUI、共享真实会话的 arm64 接入路线 | CHECK-072；专用官网 QQ 7.0.2-53644 副本、本地重签名、薄适配、原版 Framework v4.18.41、macOS 27.0 |
| Rust CLI 的 macOS 运行管理尚未完成 | 本轮调用实验脚本和原版 OneBot；接口成功不能当 CLI 平台验收 |
| 原版 napcat.cjs 不能直接用于该 Mac QQ | 当前会话类没有其假设的 create；nativeLoader.cjs 可接收已有 session/loginService |
| 会话就绪可从官方真实初始化观察 | onOpentelemetryInit 严格 true 与匹配代次；账号散列与 OneBot 身份一致，不靠假通知 |
| 10 个限定读取接口实测通过 | CHECK-072 的接口表；私聊历史跳过，写入/媒体/事件/恢复未由该实验覆盖 |
| 原 NapCat 文件保持一致 | 680 个 Framework 文件与固定官方 ZIP 字节匹配；不是修改后的 NapCat fork |
| 副本原生 hook 有签名权益要求 | 该组合需要主进程 unsigned executable memory 等权益；不等于其它版本同样可用 |
| 路径和配置不是仅靠 NAPCAT_WORKDIR 即可隔离 | QQ 还计算容器共享路径；需受控用户目录，OneBot 配置名为 onebot11.json |

当前真机基线为 v4.18.41 / 859eb530c5801577633c2c3ae871d23586edf281；固定摘要、方法和失败记录在 CHECK-072。
早期 v4.18.38 / bbd0d2aca3d470c20b33b1d4f95123335633bf26 的静态结果保留在 CHECK-068/069，不混写为同一次实验。
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
- Intel/Rosetta 不在需求范围；其它 QQ、系统、NapCat 版本和多实例/更新/长期稳定性均需独立证据。
- libproc、kqueue、Keychain、进程组和保管机制仅为平台实现候选；监听检查不等于同 socket 归属与 PID 重用防护。

## 本地适配事实

协议/大部分领域逻辑可复用；Windows Job、FILETIME、受检 HTTP/WS、凭据、QQ 发现及生命周期仍有平台依赖。
CHECK-067 已完成原生目录隔离和非 Windows 类型检查，不等于 macOS 启动支持。
技术边界在 SPEC-006/008，实施差距由 TASK-006 维护；FACT 不保存执行顺序或任务计数。
