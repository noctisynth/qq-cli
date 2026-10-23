# macOS Framework 启动入口续查

文档编号：CHECK-071。状态：来源与加载链静态核对，候选未执行。日期：2026-10-23。
关联：FACT-005、RFC-0008、TASK-006；接续 CHECK-070。用户要求继续调研如何启动，本轮未再启动/停止 QQ，未改应用入口或安装 LiteLoader。上游材料仅保存于忽略研究目录。

## 来源与固定基线

| 来源 | 提交或来源状态 | 用途 |
| --- | --- | --- |
| NapCatQQ / Mac Installer | 沿用 CHECK-068 固定提交 | 原版 napcat.cjs、nativeLoader.cjs 和 Mac 入口切换 |
| LiteLoaderQQNT | `1a8c9707c3673e0491445a1d6d7a86f0a1aa8000` | main.js 与插件装配顺序 |
| Mzdyl/LiteLoaderQQNT_Install | `32a5eeda8768039dcba525984e32026122f802e6` | install.sh 的 Mac 入口及热更新限制 |
| NapCat.Docker.Framework | `90ffc0555b39f2e6eadd56379e0e3056ea0418cc` | 无 LiteLoader 的 Linux 原生入口 |
| NapCatLinuxNodeLoader | `51e878e018e9ec8ce838d7ae0d1f0c98b04d3d4d` | 排查 NodeLoader 候选，未发现已证实 Mac 共享桌面入口 |
| Electron 环境变量文档 | 可变页面，检索日 2026-10-23 | 解释 packaged app 的 NODE_OPTIONS 限制，不作固定运行基线 |

上述新仓库文本从固定 codeload 归档读取，未执行。LiteLoader main.js SHA-256 为 `295edb2ff0f85389c13855c0452c78c83bbe9d208ea58a85737366a9be5dc4f0`；安装脚本为 `2710be96de9c864dce5257089073a61d589d401e9b7dc6c0ad3c204ab8808548`；Dockerfile 为 `316528bd7553052412767a02b0f80652191d716d2f90bad11f7449941fd5bb0b`。

## 已确认的入口机制

1. [Electron 官方环境变量说明](https://www.electronjs.org/docs/latest/api/environment-variables) 对 packaged app 的 NODE_OPTIONS 有通用限制，允许列表不包括 --require。与 CHECK-070 日志一致。因此“商店版失败，换官网版即可成功”没有依据；官网版是否受相同限制仍需对应产物验证，但不应把重试同一环境变量作为首选方案。开启 nodeOptions fuse 也不等于 packaged app 接受全部选项。未修改 fuse。
2. [NapCat 当前 Framework 指南](https://napneko.github.io/guide/boot/Framework) 仍要求已有 LiteLoader，指向 Framework ZIP 导入。该页面为可变来源；CHECK-068 的固定正文具有相同方向。不能将这份指南直接当成 v4.18.38 ZIP 可导入证据：该 ZIP 缺 manifest.json，而固定 LiteLoader 的 [loader.js](https://github.com/LiteLoaderQQNT/LiteLoaderQQNT/blob/1a8c9707c3673e0491445a1d6d7a86f0a1aa8000/src/main/loader.js) 使用 manifest.platform、injects.main 等装配插件。
3. [LiteLoader 官方安装说明](https://liteloaderqqnt.github.io/guide/install.html) 的入口方法是修改 QQ package.json.main 指向脚本，再 require LiteLoader。固定 [install.sh](https://github.com/Mzdyl/LiteLoaderQQNT_Install/blob/32a5eeda8768039dcba525984e32026122f802e6/install.sh) 的 patch_resources 实现这一机制。其 patch_macos_qq_hot_update 明确提示不能处理 Mac 热更新，并建议更新基础包或使用商店版；这反而不能支持“商店版天然不能加载”的概括。
4. 固定 LiteLoader [main.js](https://github.com/LiteLoaderQQNT/LiteLoaderQQNT/blob/1a8c9707c3673e0491445a1d6d7a86f0a1aa8000/src/main.js) 先安装 hook、装配插件，再调用 QQ 的 application.asar/app_launcher/index.js，并安排修正运行期 launcher.main。它提供保留原始桌面启动链的源码结构，不等于已验证当前 QQ 与 NapCat 包组合。没有运行 LiteLoader。
5. 官方 Mac Installer 的 createLoader 改写应用入口，在 Shell import 与原 app_launcher 之间互斥选择，仍不是 Framework 共享窗口方案。不能把 Shell 的启动命令加上 --no-sandbox 当作 Framework 接入步骤。
6. 固定 [Dockerfile](https://github.com/NapNeko/NapCat.Docker.Framework/blob/90ffc0555b39f2e6eadd56379e0e3056ea0418cc/Dockerfile) 在 Debian/Linux QQ 上通过 LD_PRELOAD 装载 libnapiloader.so，并以 NAPCAT_EXTERNAL_SCRIPT_PATH 指向 nativeLoader.cjs。这是上游现存的无 LiteLoader Framework 链路，但依赖 Linux 原生加载器，不是 Darwin 动态库，不能靠改环境变量名移植。

## 原版 CJS 的约束与可研究候选

固定 napcat.cjs 只安装 process.dlopen 拦截，等待 wrapper.node 被加载，再代理 session/loginService 并初始化 Framework。它本身不启动 QQ 原始入口。nativeLoader.cjs 则要求调用方提供现有 session/loginService/registerCallback。由此可排除“直接把 package.json.main 指向 napcat.cjs 就能保留桌面”的未经证明说法。

候选链为：QQ 的 package.json.main → 一段入口胶水 → 原版 napcat.cjs → QQ 原始 app_launcher。这沿用了已观察到的入口改写机制及上游 CJS，但组合本身是本项目推断，不是已发现的原版 Mac 启动器。实施将需要自有入口胶水，与当前“不开发加载器”边界有待明确，不能通过改称配置来自动视为已授权。

该候选尚有实质时序问题：CHECK-070 日志显示 QQ 自身在早期 preload wrapper.node；这不证明后续 JS require 会再次经过 process.dlopen。原版 CJS 必须实际捕获该加载及 session 初始化，不能由文件执行或 WebUI 出现推定。另有异步 NCoreInitFramework/回调登记时序、应用签名/权限、热更新入口及恢复、多实例共用数据边界。Shell 的正常部署不解决这些问题。

若后续决定允许仅入口胶水的实验，应先冻结完整元组和备份/恢复方案，核验没有其它 QQ 会话受影响，再检查 hook 捕获、Framework 初始化、官方窗口、用户完成登录后的同一身份及受限 API。不得为赶上时序而自动增加二进制 hook、修改 NapCat、替换/重签应用或采用 LiteLoader。试验失败应保留日志并恢复本次确实修改的入口；不调用安装器的 killall。

## 补充风险线索与证据边界

- [NapCatQQ issue 2066](https://github.com/NapNeko/NapCatQQ/issues/2066) 是用户在 Mac arm64、QQ DMG 6.9.82-40990、NapCat Shell 4.18.28 上报告的 Packet 自检失败；不是本项目复现，也不是 v4.18.38 结论。它说明“hook 初始化成功”仍不足以证明 Packet 操作通过。
- [Mac Installer issue 12](https://github.com/NapNeko/NapCat-Mac-Installer/issues/12) 报告热更新入口未恢复导致启动错误；固定安装器现有 guard 不能用旧 issue 推定仍有同一错误。基础包与热更新包都需纳入未来入口恢复验证。
- 搜索还出现同名 napcat.app，它明确不是 NapCatQQ，未作为技术证据；NapCatReforged 已归档，未把旧 loader 名称视为当前 Mac 启动器。

结论：可确定 Mac 上常见的装载机制是应用入口改写，不能确认原版 Framework 已有符合当前全部约束的 Mac 启动器。当前最具体的下一步是评估“仅入口胶水”的有界实验是否允许，而非更换 QQ 发行版后重复 NODE_OPTIONS。未证明完整 Mac 接入，更未完成业务兼容验收。
