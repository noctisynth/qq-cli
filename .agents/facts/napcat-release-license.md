# NapCat 固定版本、许可与产物核对

文档编号：FACT-002。状态：Static-verified。更新：2026-10-16。
证据：EVIDENCE-001、DATA-001、CHECK-002。静态核对不构成运行兼容性或法律结论。

## 版本和附件

固定审计 v4.18.38，发布于 2026-10-12 04:36:31 UTC。
注解 tag 对象 e0f817edf1a32d0825d433bbf9c8e9dc06b32873，指向提交
bbd0d2aca3d470c20b33b1d4f95123335633bf26。
Release ID 410166712，Framework asset ID 632672494；API 标记 release 非 immutable。
标签和附件名不足以锁定内容，后续同时检查提交与 SHA-256，不随 latest 自动替换。

实际下载 NapCat.Framework.zip，36,728,573 字节，680 个非目录成员。
实际 SHA-256 为 45ef7f46aa78588c636044b6226b4b1847e31287ab6ec36b2d48adaa9785097b，
与 GitHub API digest 一致。无重复或目录穿越成员名，未执行附件代码。
包中 package.json 的 version 为 0.0.1，不能代替发布标签作为版本依据。

## 许可观察

固定提交根 LICENSE 为自定义 Limited Redistribution License for NapCat，并非 MIT/Apache。
条款要求未授权的使用、复制、修改或分发取得作者明确许可；同时又规定带完整许可和署名的再分发，
并限制修改版公开与商业用途。如何适用于独立加载器及外部 CLI 有解释边界，不自行推导无限授权。
同提交 README 还要求基于 NapCat 代码开发的其他项目取得作者授权，第三方部分分别遵循原许可。
未修改产物、用户自行下载、单独开源加载器或外部 HTTP 调用，都不能自动视为授权已解决。
当前标记“集成和分发授权未确认”；本轮只记录研究，不复制实现进入受跟踪源码、不打包附件、不 fork。
未联系作者；后续取得适用依据再明确可用范围。

Framework ZIP 根目录没有 LICENSE 或 README。依赖中的第三方 LICENSE 不替代 NapCat 根许可；
尚未完成全部三方及原生依赖许可、来源和通知文件审计。

## 实际发布入口

| 产物 | 静态内容 | 边界 |
|---|---|---|
| napcat.cjs | Hook process.dlopen，代理会话与登录服务 | 提前加载入口，不是已有 PID 的控制 API |
| napiLoader.bat | 把 QQ、napiloader.dll、nativeLoader.cjs 传给 napimain.exe | 已有不经 LiteLoader 脚本的原生加载候选路径 |
| nativeLoader.cjs | 导出 initializeNapCat(session, loginService, registerCallback) | 捕获异常后只记录日志，不能以返回作为成功证明 |
| napcat.mjs | 实际导出 NCoreInitFramework、NapCatFramework、getWebUiUrl | 实际包存在接口，不代表稳定公共 SDK |

此前“必须先自行开发加载器”的假设不成立：应先评估原版原生入口。
也不能升级为“原版已经支持当前 QQ、无需适配”。不在普通 Node 进程 import 打包文件做烟测，
因为它依赖 QQ 上下文、内部原生模块及 ABI，导入可能产生副作用。

EXE/DLL 的 PE machine 均为 0x8664（x64），与 tag 中二进制逐字节一致；
两份 BAT 在统一换行后一致，napcat.cjs 和 nativeLoader.cjs 与固定源文件逐字节相同。
对应 napcat-napi-loader 目录只带原生二进制和 BAT，没有可重建的加载器实现；
未完成可复现构建、签名信任或实际行为证明。
DLL 内有 getNTWrapperSession、NodeIKernelLoginService/get、onOpentelemetryInit 和
initializeNapCat 字符串及原生 Hook 框架线索；EXE 有创建进程和加载 DLL 的符号线索。
它们支持“捕获官方启动会话”的意图推断，不能单凭字符串证明对象、时序或线程正确。
不得把另一 Windows Boot 仓库源码当作本包原生加载器的对应实现。

## 生命周期缺口

- 初始化等待登录成功，再等待会话初始化回调；时序错位可能卡住。主要完成分支为
  onQRCodeLoginSucceed，不能保证自动/快速登录都进入相同路径。
- 会注册登录连接事件、请求二维码并启动原生 packet 处理，并非完全被动只读旁路。
- process.execPath 用于推断 QQ 信息；NAPCAT_WORKDIR 可分离部分配置/日志/缓存，不隔离全部 QQ 数据。
  默认 config 含 o3HookMode；关闭可选 bypass 不等于无 Hook。
- 断线调用 requestRestartProcess；实际 bundle 未发现 setRestartProcessCall 调用，原生层是否补足未知。
- 产物含退出进程、更新及登录管理 API；不能向 Agent 暴露全部管理能力。

结论：真实包有无 LiteLoader 接入所需的文件和函数；窗口保留、共享会话和不干扰体验仍待运行验收。
许可与原生加载器可审计性是进一步集成的未解决条件。

2026-10-18 离线补查同一已固定 ZIP 的目录元数据：806项（含目录），展开总量118,912,323字节，最大成员23,233,912字节。
此次仅读取ZIP元数据，没有提取或执行；数值用于本项目有界解包设计，不改变适用授权与运行兼容结论。
后续来源追溯见 FACT-003、DATA-002：已找到预编译文件首次引入和完整主线迁移记录，
仍未确认源码到 PE 的生成链；启动器外围维护不能当作原生定位规则或新 QQ 版本的维护证明。
