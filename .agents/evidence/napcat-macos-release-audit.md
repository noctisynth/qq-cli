# NapCat macOS 固定发布资产复核

文档编号：CHECK-068。状态：静态资产/源码核验，非运行验收。日期：2026-10-23。
关联：FACT-005、RFC-0008、TASK-006。基线为 NapCat v4.18.38 / bbd0d2aca3d470c20b33b1d4f95123335633bf26。

## 方法与来源

从官方固定 release URL 下载原版包到忽略研究目录。先计算 SHA-256，与 GitHub [附件清单](https://github.com/NapNeko/NapCatQQ/releases/expanded_assets/v4.18.38) 公布摘要比对，再用 Python zipfile 检查 CRC、成员与内容。Darwin 文件读取 Mach-O 头和 LC_BUILD_VERSION，并以系统 otool -l 独立复核；只检查二进制，不 dlopen、不执行 JS/安装器/QQ。

Framework ZIP：36,728,573 字节，680 个文件，CRC 全部通过；SHA-256 为 `45ef7f46aa78588c636044b6226b4b1847e31287ab6ec36b2d48adaa9785097b`，与官方公布值及项目固定清单一致。

Shell ZIP：35,884,742 字节，685 个文件，CRC 全部通过；SHA-256 为 `735f2e633247df3ffd6e8ee8b84d7039c315b2655f5807e5f0029f93d2c30016`，与官方公布值一致。六个 Darwin 模块与 Framework 包逐字节一致，下面的架构/minos 核对同样适用于 Shell。两包以及六个组件的大小、摘要、架构记录见同目录 napcat-macos-assets.json（本证据附属数据）。

Shell 的 loadNapCat.js 只选择并导入 napcat.mjs；实际 bundle 在末尾主动执行进程启动逻辑，只导出 restartWorker，没有 Framework 的三个导出。Mac 安装器生成的选择入口与该包的 Shell 职责一致，不能把 Shell napcat.mjs 换称为共享桌面会话的 Framework 入口。

## Framework 包内事实

| 组件 | 实际架构 | Mach-O 声明 minos | SDK |
| --- | --- | --- | --- |
| ffmpegAddon.darwin.x64.node / arm64.node | x86_64 / arm64 | 11.0 | 15.5 |
| napi2native.darwin.x64.node / arm64.node | x86_64 / arm64 | 15.0 | 15.5 |
| MoeHoo.darwin.x64.node / arm64.node | x86_64 / arm64 | 15.0 | 15.5 |

六个文件均为实际 Mach-O 64 位文件，不是占位文件或 LFS 指针；以 Git blob SHA-1 编码计算后与固定提交 tree 对应文件一致。架构由 CPU type 读取，不依赖文件名。minos 是二进制声明，不等于本项目已测试的系统版本；尚未验证低版本实际失败方式或部分组件缺失时的业务范围。

实际 napcat.mjs 中 Napi2NativeLoader 明确允许 darwin.x64/darwin.arm64，按进程 platform/arch 选择模块；QQBasicInfoWrapper 和 loadQQWrapper 有 macOS QQ Helper/Resources/app 路径分支。故“Framework 内部没有 macOS 适配”不成立；但这不解决谁先在 QQ 内加载 Framework 的问题。

napcat.cjs 与 nativeLoader.cjs 和固定源码逐字节一致。前者在已执行的前提下拦截 wrapper.node、创建/代理 session 和 loginService；后者导出 initializeNapCat，接收外部提供的 session/loginService，调用 NCoreInitFramework。实际 napcat.mjs 导出 NCoreInitFramework、NapCatFramework、getWebUiUrl。不能把导出函数存在当作独立可启动程序。

根启动资产为 napiLoader.bat、napiLoader-debug.bat、napimain.exe、napiloader.dll、两份 CJS；BAT 读取 Windows 注册表后按三参数调用 EXE。固定包所检根启动资产中没有 macOS 启动脚本或可执行加载器。包根也没有 manifest.json；现有通用 Framework 指南不能作为这个固定 ZIP 在 LiteLoader 下直接导入成功的运行证据。本轮没有安装 LiteLoader，也未证明其它位置绝无可用入口。

## 固定官方文档与安装器

文档站固定提交 `3c04f70a9e4ff84ab4135b9cf06f29608539b16c` 的实际 HTML 已读取，补齐初轮只固定 tree 的缺口：

- [Shell.html](https://github.com/NapNeko/napneko.github.io/blob/3c04f70a9e4ff84ab4135b9cf06f29608539b16c/guide/boot/Shell.html)：macOS 12+、Intel/Apple Silicon，指向 Mac 安装器并要求修改入口。
- [Framework.html](https://github.com/NapNeko/napneko.github.io/blob/3c04f70a9e4ff84ab4135b9cf06f29608539b16c/guide/boot/Framework.html)：已有 LiteLoader 环境、导入 Framework 包。
- [release.html](https://github.com/NapNeko/napneko.github.io/blob/3c04f70a9e4ff84ab4135b9cf06f29608539b16c/guide/boot/release.html)：区分 Shell 通用包与 Framework 包。

HTML SHA-256 分别为 `fb3c8c34c12122dbe90647fbfa933ef3b9e5851221e30c58a25554eeab292faa`、`3f185ba2ca55a58ff0834f6edfc12754db7c521cdcfa4566e568d103d445b3a4`、`2b4a29abc7091c1bef6f058456f0d38f88005b6ec808c48b2663817d0d4244f0`。

安装器固定提交继续为 `accafd630b55656035a24486f56b3c08cb0505d1`。Network.swift 实际从 latest release 元数据选择 NapCat.Shell.zip 并读取 digest，不能把该自动 latest 路径当成固定 v4.18.38 的运行验证。Utils.swift 生成的入口根据 --no-sandbox 在 Shell 和原版 app_launcher 之间互斥选择；ContentView.swift 在两种启动操作前调用按名称终止 QQ 的函数。与 FACT-005 初轮观察一致。

## 判断边界

已确认的是：官方有 macOS Shell 部署说明，固定 Framework 含真实 Darwin 组件及 macOS 代码分支。尚未确认的是：符合本项目既有约束的 macOS Framework 外部加载入口，以及特定 QQ/系统/架构组合的实际共享会话运行。
指南的 macOS 12+ 与已检查组件声明的 minos 15.0 不一致，不能无条件沿用该系统支持下限。不能由静态头直接断言所有功能在 12–14 都不可用，也不能把源码分支、ZIP 完整性或导出接口升级成运行兼容结论。
