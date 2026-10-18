# 证据登记

文档编号：EVIDENCE-001。状态：Active。更新：2026-10-19。

CHECK-022新run元数据内核：g1认领先于n1/登记，稳定实例与账号清空、签名来源/内部键恢复、登记阻断后复用、回滚/缺来源/缺原停止证明拒绝、4线程唯一计划、离线request查询和后继默认退役，共10项通过；完整267项及clippy/fmt/release/架构/196文件源码包通过。纯原创状态/预启动取消证明，无真实QQ动作；公开start/restart及Inputs组合验收仍待。

CHECK-020停止确认增量：原部分字段判定下账号变化回归失败；完整登记快照后15类变化保持原登记/证明，正常及复用确认的落盘失败均unknown且可恢复。新增4项、停止内核10项及完整257项回归、clippy/fmt/release/架构/192文件源码包通过。无生产故障注入接口或真实QQ操作，不代替完整生命周期故障矩阵，业务62/95不变。

CHECK-021原生凭据协调有界验收：进程内候选被压力否定撤回；跨进程命名mutex最小权限、非继承、有限等待和线程绑定释放经2项原创异常测试；未包装的原并发6项连续10轮及完整253项离线回归、clippy/fmt/release/架构/190文件源码包通过。底层根因与完整凭据/生命周期矩阵未定，业务62/95不变，无真实QQ操作。

CHECK-020公开remove已接签名停止前提、x1注销意图/完成恢复与精确默认指针退役；17项最终定向、clippy/fmt/release/架构/187文件源码包通过。完整回归在既有stop凭据恢复用例失败，父进程恢复写入后也读不到，最小独立诊断脱离Job仍见删除/读取不一致，根因未定，不称完整通过。业务62/95，完整凭据故障、真实覆盖和完整生命周期矩阵仍待验收。

CHECK-020公开stop已接合：完整243项离线回归及clippy/fmt/release/架构/183文件源码包通过，含5项原创CLI及原3项status，覆盖并发/歧义/复用、保留凭据与记录、另一进程组存续以及缺原Job保留unknown。一次exit4未定位，结构化断言和连续重跑结果一并记录，不承诺消除间歇性问题。业务61/94，真实QQ和完整生命周期仍待验收。

CHECK-020新增停止内核证据：7项原创测试、库级58项通过，涵盖签名意图/完成恢复、启动认领及目标变化拒绝、QUERY权限隔离和两组原创子进程互不影响。Job计数清空与进程退出信号分别观察；公开stop及真实QQ生命周期仍待验收，业务60/93不变。

CHECK-019追加空状态status补修：not_started纳入Schema并明确未验证身份，13项CLI定向及全目标clippy通过；不创建状态/凭据，业务计数60/93不变，完整231项为修正前基线。

CHECK-019补公开待登录status接合：3项独立CLI/原服务及1项身份分类/版本先行单元验证提示与身份分离、初始化/版本错误、坏HTTP/身份不降级、2FA/拒绝/并发换run及缺原材料不修补，实际输出经独立schema验证；最终完整231项离线回归及fmt/clippy/release/架构/174文件源码包通过。无真实QQ操作；完整登录恢复保持未验收。

CHECK-019开始记录登录观察实现：固定协议4项原创测试及全目标clippy通过，阶段/布尔提示与身份分离；尚无受检控制传输或CLI接合，不扩大真实操作授权。
CHECK-019现已补受检控制HTTP：新增2项独立同Job管理服务测试，完整226项回归及fmt/clippy/release/架构/169文件源码包通过；只读控制能力/秘密恢复及公开CLI仍待接，不作为真实原版或登录恢复验收。
CHECK-019再补只读原控制能力/秘密恢复：11项定向及完整227项回归、fmt/clippy/release/架构/171文件源码包通过。已有j1/t1/r1/marker/vault相互核对，缺失不修补；尚未接公开status，不提升真实兼容或完整登录验收。

CHECK-018核对FACT-002固定LICENSE/README原文和Git对象摘要，区分外部原创API客户端、官方源下载、本地运行及复制修改分发；许可适用未知不再成为安装/启动/技术canStart门禁，兼容及归属保护不变。220项离线测试及fmt/clippy/release/架构/165文件源码包检查通过，不扩展真实QQ操作授权或提升完整交付验收。

CHECK-017登记模块质量整改：61根模块→9子系统、app入口747→135行，应用用例/生命周期展示/诊断及媒体依赖边界、私有run_job拆分和单元测试迁移；完整220项测试与迁移前逐项对应、fmt/clippy/release/包白名单及结构检查通过。无真实QQ操作，不计新增业务能力。

CHECK-016登记同一固定提交的WebUI鉴权/登录观察协议：11个固定blob、匹配摘要包符号核对，区分WebUI凭据与QQ身份；只读观察候选不调用快速/密码/刷新/设备接口。实际控制传输/CLI及真实原版仍未验收，不提升兼容支持。

CHECK-015登记固定预登录生命周期源码核对：提交bbd0d2aca3d470c20b33b1d4f95123335633bf26的Framework/WebUI/适配器/OneBot HTTP五个blob及匹配DATA-001摘要发布包交叉观察。WebUI先调用、登录后创建OneBot；同进程仅架构推断，端口可漂移，实际PID/Job/登录兼容仍未验收。来源与固定blob详见CHECK-015，不添加可变基线或真实会话结论。

公开材料检索日期为 2026-10-16。SRC-001 至 SRC-009 为早期可变来源，仅作背景；
本轮固定 Framework 审计使用下面 PIN 系列和 DATA-001，不把旧启动器来源当作新原生加载器实现。

| 编号 | 公开来源 | 支持内容 |
|---|---|---|
| SRC-001 | https://github.com/NapNeko/NapCatQQ/blob/main/packages/napcat-shell-loader/launcher.bat | Windows Shell 启动参数 |
| SRC-002 | https://github.com/NapNeko/NapCat-Windows-Boot/blob/main/main/main.cpp | 创建新进程和加载 DLL 的公开实现 |
| SRC-003 | https://github.com/NapNeko/NapCatQQ/blob/main/packages/napcat-shell-loader/qqnt.json | Shell 入口配置 |
| SRC-004 | https://github.com/NapNeko/NapCatQQ/blob/main/packages/napcat-shell/base.ts | Shell 引擎、登录和会话初始化 |
| SRC-005 | https://github.com/NapNeko/NapCatQQ/blob/main/packages/napcat-framework/napcat.cjs | 桌面会话代理 |
| SRC-006 | https://github.com/NapNeko/NapCatQQ/blob/main/packages/napcat-framework/nativeLoader.cjs | 注入初始化对象的函数入口 |
| SRC-007 | https://github.com/NapNeko/NapCatQQ/blob/main/packages/napcat-core/apis/msg.ts | 内部消息服务调用 |
| SRC-008 | https://github.com/NapNeko/NapCat-Windows-Boot/blob/main/hook/hook.cpp | 启动读取重定向和 Hook |
| SRC-009 | https://napneko.github.io/guide/boot/Framework | 官方现有 LiteLoader 安装途径 |

## 固定版本静态审计

固定提交：bbd0d2aca3d470c20b33b1d4f95123335633bf26；完整摘要见 DATA-001，方法见 CHECK-002。

| 编号 | 固定公开来源 | 支持内容 |
|---|---|---|
| PIN-001 | https://github.com/NapNeko/NapCatQQ/releases/tag/v4.18.38 | 发布标签和附件 |
| PIN-002 | https://api.github.com/repos/NapNeko/NapCatQQ/releases/410166712 | release/asset ID、长度、平台 digest |
| PIN-003 | https://github.com/NapNeko/NapCatQQ/blob/bbd0d2aca3d470c20b33b1d4f95123335633bf26/LICENSE | 自定义根许可 |
| PIN-004 | https://github.com/NapNeko/NapCatQQ/blob/bbd0d2aca3d470c20b33b1d4f95123335633bf26/README.md | 独立项目授权与混合许可限制 |
| PIN-005 | https://github.com/NapNeko/NapCatQQ/tree/bbd0d2aca3d470c20b33b1d4f95123335633bf26/packages/napcat-napi-loader | 原生二进制、启动 BAT；目录无可重建实现 |
| PIN-006 | https://github.com/NapNeko/NapCatQQ/blob/bbd0d2aca3d470c20b33b1d4f95123335633bf26/packages/napcat-framework/nativeLoader.cjs | 对象接入和异常处理 |
| PIN-007 | https://github.com/NapNeko/NapCatQQ/blob/bbd0d2aca3d470c20b33b1d4f95123335633bf26/packages/napcat-framework/napcat.ts | 初始化和生命周期 |
| PIN-008 | https://github.com/NapNeko/NapCatQQ/blob/bbd0d2aca3d470c20b33b1d4f95123335633bf26/packages/napcat-framework/vite.config.ts | 产物装配和依赖 |

DATA-001 保存 ZIP 和关键成员的摘要，不保存上游正文或二进制。
原始下载与只读检出属于忽略研究材料；未执行上游代码、重启 QQ、发信或联系作者。
历史 SRC-002 与发布启动器的匹配仍未完成；本轮只证明 Framework 相关选定文件的一致性。
初始化见 CHECK-001；本登记不保存聊天、账号或本机目录。

## 原生加载器历史与版本线索

追溯结果见 FACT-003、DATA-002、CHECK-003。主线观察固定到
810052c605feb5044a080adfe99ba2fa69b7b351；不改变 v4.18.38 的发布基线。

| 编号 | 公开来源 | 证据边界 |
|---|---|---|
| TRACE-001 | https://github.com/NapNeko/NapCatQQ/commit/080e4d198fb331fc05e0c5ad79ccd7bf42e451e6 | 引入预编译原生文件和脚本；本轮通过 Git 对象核对 |
| TRACE-002 | https://github.com/NapNeko/NapCatQQ/commit/ed19c52f257d19b021ee71f4d66a49eaf7d59dcc | 目录迁移，旧/新 blob 一致 |
| TRACE-003 | https://github.com/NapNeko/NapCatQQ/commit/9109b8cfe2f46bc6f4f80cdeec8d1621d06e186d | BAT/CJS 路径修复，非目标二进制更新 |
| TRACE-004 | https://github.com/NapNeko/NapCatQQ/commit/ec6aadaeea38ed3bca67b82bab3d7a9c0f91b8d8 | QQ 9.9.36 Shell 修复，不能作为 Framework 支持依据 |
| TRACE-005 | https://github.com/NapNeko/NapCatQQ/blob/bbd0d2aca3d470c20b33b1d4f95123335633bf26/.github/workflows/release-publish.yml | 原生文件复制打包及 Shell 专属验证边界 |
| TRACE-006 | https://github.com/NapNeko/NapCat-Native/tree/9df503de54401b4484e60d8090c6dd05cb94775a | 相邻 node-gyp addon，未建立与目标 PE 的映射 |
| REPORT-001 | https://github.com/shuakami/qq-chat-exporter/issues/485 | 下游 QQ 9.9.26-44343 的用户报告，不是本项目或原版同会话验收 |
| REPORT-002 | https://github.com/shuakami/qq-chat-exporter/issues/332 | 界面/接收线索，未给明确 QQ 和原生摘要 |
| REPORT-003 | https://raw.githubusercontent.com/shuakami/qq-chat-exporter/master/scripts/build-framework-plugin.py | 2026-10-16 读取的可变脚本，显示下游重写 BAT；不当作历史包内容证明 |

issue、文档和 REPORT-003 为可变来源，只登记日期与摘要，不保存原日志/用户标识。
历史范围内未找到源码/构建证明不等于其他位置或私人仓库不存在。没有联系作者或执行目标代码。

## 文档历史、维护者答复与安全

结果见 FACT-004、CHECK-004。文档观察提交为 3cf609b5bc96d56df1dcf9cb3388a6b90665febf。

| 编号 | 公开来源 | 支持内容与限制 |
|---|---|---|
| HISTORY-001 | https://github.com/NapNeko/NapCatDocs/commit/c13e2b9bed8add13d816c69049affaf425b06377 | 2025-05-12 增加 LiteLoader 维护警告 |
| HISTORY-002 | https://github.com/NapNeko/NapCatDocs/commit/d1c261af78d53e57bd594ed483392a5a3bf08ace | 删除旧 Installer，不是 napiLoader |
| HISTORY-003 | https://github.com/NapNeko/NapCatDocs/commit/5f07655fc59b24f58b8adad79699d998a0a34619 | 简化指南并保留 LiteLoader；无原生缺席解释 |
| HISTORY-004 | https://github.com/NapNeko/NapCatQQ/issues/1375 | 2025-11-10 Member 回复弃用 Once/OneKey，推荐原生 BAT |
| HISTORY-005 | https://github.com/NapNeko/NapCatQQ/issues/1398 | 2025-12-05 Member 回复原生入口可有头加载 |
| SAFETY-001 | https://github.com/LiteLoaderQQNT/LiteLoaderQQNT | 归档及账号风险警告，2026-10-16 读取的可变页面 |
| SAFETY-002 | https://napneko.github.io/other/security | 官方讨论账号影响/风控，不是免封声明；历史配置不直接移用 |
| SAFETY-003 | https://github.com/NapNeko/NapCatQQ/discussions/517 | 历史 Hook/反检测说明，不代表当前效果 |
| SAFETY-004 | https://github.com/NapNeko/NapCatQQ/commit/62c92463681365feeffe634ec7d55eb3dbcf5f50 | 提交主题说明反检测意图 |
| SAFETY-005 | https://github.com/NapNeko/NapCatQQ/commit/5c72f771c37273c5c3e91a655b777ae991818a7f | 说明两种模式初始化调用 enableAllBypasses |
| SAFETY-006 | https://github.com/NapNeko/NapCatQQ/commit/b9f61cc0eee02465e9a69b3d99675c304265af35 | 配置反检测及 Bypass 崩溃恢复的提交说明 |
| SAFETY-007 | https://github.com/NapNeko/NapCatQQ/blob/bbd0d2aca3d470c20b33b1d4f95123335633bf26/packages/napcat-shell/base.ts | 固定 Shell 调用点；Framework 见 PIN-007 |
| SAFETY-008 | https://github.com/NapNeko/NapCatQQ/blob/bbd0d2aca3d470c20b33b1d4f95123335633bf26/packages/napcat-core/packet/handler/napi2nativeLoader.ts | 原生导出类型和加载逻辑，不是原生实现 |
| SAFETY-009 | https://github.com/NapNeko/NapCatQQ/blob/bbd0d2aca3d470c20b33b1d4f95123335633bf26/packages/napcat-core/helper/config.ts | Bypass 配置字段，不证明所有平台默认效果 |

## CLI 协议与工具链依据

PLAN-001、SPEC-004 的核对方法见 CHECK-007；CHECK-005、CHECK-006 为已有实验依据，未新增运行请求。
以下源码 URL 的路径通过固定 Git tree 核对，具体字段同时读取 DATA-001 对应产物；不复制实现。

| 编号 | 公开来源 | 支持内容与限制 |
|---|---|---|
| CLI-001 | https://github.com/NapNeko/NapCatQQ/blob/bbd0d2aca3d470c20b33b1d4f95123335633bf26/packages/napcat-onebot/action/go-cqhttp/GetFriendMsgHistory.ts | 私聊历史参数、取最近消息及构造消息 ID |
| CLI-002 | https://github.com/NapNeko/NapCatQQ/blob/bbd0d2aca3d470c20b33b1d4f95123335633bf26/packages/napcat-onebot/action/msg/GetMsg.ts | 消息 ID 映射依赖，不承诺跨重启使用 |
| CLI-003 | https://github.com/NapNeko/NapCatQQ/blob/bbd0d2aca3d470c20b33b1d4f95123335633bf26/packages/napcat-onebot/action/msg/SendPrivateMsg.ts | 私聊发送入口，不证明上游幂等或对方收悉 |
| CLI-004 | https://github.com/NapNeko/NapCatQQ/blob/bbd0d2aca3d470c20b33b1d4f95123335633bf26/packages/napcat-onebot/action/system/GetStatus.ts | online 及固定 good=true，非全面健康检测 |
| CLI-005 | https://github.com/NapNeko/NapCatQQ/blob/bbd0d2aca3d470c20b33b1d4f95123335633bf26/packages/napcat-onebot/action/system/GetVersionInfo.ts | NapCat app_version 与 OneBot protocol_version |
| CLI-006 | https://github.com/NapNeko/NapCatQQ/blob/bbd0d2aca3d470c20b33b1d4f95123335633bf26/packages/napcat-onebot/action/system/GetLoginInfo.ts | 当前账号信息，不自动选定目标账号 |
| TOOL-001 | https://nodejs.org/en/about/previous-releases | 历史 Node 候选依据，已被用户 Rust 决定替换；未安装工具链 |
| TOOL-002 | https://nodejs.org/docs/latest-v24.x/api/test.html | 历史 Node 候选依据，不再作为当前测试路线 |
| TOOL-003 | https://doc.rust-lang.org/cargo/reference/manifest.html | Cargo manifest、edition 与 rust-version；工程验证另见 CHECK-008 |
| TOOL-004 | https://docs.rs/clap/latest/clap/ | Rust 命令解析候选，2026-10-17 可变 latest 文档 |
| TOOL-005 | https://docs.rs/reqwest/latest/reqwest/blocking/struct.ClientBuilder.html | blocking 客户端代理/重定向/重试及超时配置；不是组合构建通过证据 |

## Agent CLI 设计参考

2026-10-17 读取；以下均为可变网页，仅支持设计参考，不作为上游固定基线或本工具宿主兼容证据。

| 编号 | 公开来源 | 支持内容与限制 |
|---|---|---|
| AGENT-001 | https://github.com/larksuite/cli | 官方 README 的 schema、JSON、分页、dry-run 和 Agent 指引；不复制实现或特殊命令前缀 |
| AGENT-002 | https://opencode.ai/docs/skills/ | SKILL.md 按需发现/加载；本工具导出及实际宿主验收尚未实现 |

PLAN-001 的扩展 API 表为候选映射，除既有实验外不代表运行验证；详细契约先核对再实现，见 TASK-002。

## 运行托管设计补充

2026-10-17 再次静态读取 DATA-001 对应原版 napiLoader.bat 与 nativeLoader.cjs：
BAT 定位本机 QQ 并将 QQ、DLL、脚本传给 napimain；CJS 导出 initializeNapCat 并调用 NCoreInitFramework。
这支持“外部编排原版入口”的候选，不证明配置隔离、命令行快速登录或进程树管理已可用。
公开可变参考 https://napneko.github.io/guide/boot/Shell-Linux-SemiAuto 提示快速登录失败仍需扫码；
该页针对 Shell/Linux，不能把参数或运行支持直接移用于 Windows Framework。
RFC-0003 明确托管目标、登录挑战和活跃窗口保护，待固定产物逐项验证，不新增真实请求。

## 固定版本、兼容性与身份 UX

CHECK-011 记录 2026-10-17 的文档/静态复核，依据 FACT-002、FACT-003 与原版 BAT，不运行实例。
同时读取以下可变官方网页：

| 编号 | 公开来源 | 支持内容与限制 |
|---|---|---|
| UX-001 | https://github.com/NapNeko/NapCatQQ/releases/tag/v4.18.38 | 指定 release/tag 与提交；网页不代替已登记产物摘要，也未给完整原版兼容矩阵 |
| UX-002 | https://napneko.github.io/guide/boot/Framework | 现行指南要求确认加载框架支持 QQ，主要描述 LiteLoader，不能直接当作本方案原生入口支持证明 |
| UX-003 | https://napneko.github.io/guide/boot/Shell | Shell 指南与兼容信息，启动模式不同，不移用为 Framework 原生加载兼容保证 |

版本选择和兼容证据分级是本项目设计，不是上游已有相同 CLI 或完整判定器。
后续用户确认简化 UX：不公开 profile/托管模式/绑定配置，自动单实例、多实例显式选择，安装/实例分别管理。
实际账号与变化检测属于内部准确性要求，CHECK-011 已同步；未新增真实运行证据。

## 运行前分层预检

CHECK-014 登记2026-10-18的QQ软件元数据合成布局、PE/摘要、精确兼容键、doctor假探针和Windows原生系统信息验证。
原版BAT与QQBasicInfoWrapper/getQQVersionConfigPath/getQQPackageInfoPath/getMajorPath的静态观察基于FACT-002固定产物/提交；不复制实现，不执行QQ。
框架实测、静态检查和Shell证据不能互换；当前没有完整真实Framework兼容元组。API来源与本机依赖验证边界见 CHECK-014。

事件归一化和固定发布bundle字段补充见 CHECK-013、FACT-001；仅合成帧和静态检查，未建立真实WS事件验收。

私有运行材料准备见CHECK-013（2026-10-18）：签名认领、整组受保护凭据恢复、受管暂存中断前缀修复、Win32不覆盖目录发布及准备态业务隔离。包含随机临时原生凭据命名空间和合成文件布局，不启动QQ，不作为PID/监听归属、运行后恢复或完整生命周期证据。

启动意图内核见CHECK-013（2026-10-18）：s1签名run认领先于请求索引、prepared到starting及一次假调度；7项测试覆盖线程/独立进程竞争、主动退出、中断、冲突、损坏、回执失败及prepared崩溃窗口换键拒绝。未启动原生加载器，未证明实际进程/监听归属或完整杀进程/落盘矩阵。

持句柄进程观测见CHECK-013（2026-10-18）：2项原创Windows测试只观测自有子进程，验证原始创建值/映像复核及退出码259仍识别已退出。无真实QQ探测；不以映像路径观测证明实际加载内容、父子归属或监听归属。

命名Job见CHECK-013（2026-10-18）：2项Windows原生测试验证私有随机创建/只读重开/自有成员及最后句柄关闭不终止进程，但名称已不可重开；否定“只保存名称即可跨CLI恢复”的候选推断。只在隔离native_job模块允许有安全注释的unsafe，其他代码deny；原子根创建见后项，加载器后代保管或真实QQ子树仍未验收。

原子根进程创建见CHECK-013（2026-10-18）：3项测试覆盖显式UTF16参数/环境、创建即入Job、仅继承QUERY副本及NUL流、根进程保留名称及创建前拒绝/原生失败；并发回归促使元数据查询失败后按同一对象确认退出。未运行NapCat或QQ，原版加载器后代保管和公开start门禁仍待完成。

同Job后代只读保留见CHECK-013（2026-10-18）：2项原生测试核对完整身份、同一目标句柄成员、QUERY-only/noninheritable及父退出后的正反对照；记录最初误继承使对照失效及修正方法。一次性日志新增3项定向测试、接入后的原生链路及完整178项回归通过；完整生命周期和真实QQ兼容仍待完成，未操作真实QQ。

已连接TCP归属见CHECK-013（2026-10-18）：3项测试覆盖IPv4/IPv6、严格四元组与服务器PID、Pinned/Job复核、非成员/错误服务/截止拒绝及零应用数据；系统表其他行不输出或持久。受检HTTP见后项，托管/WS凭据路径仍待接合，未调用真实QQ API。

受检HTTP Backend见CHECK-013（2026-10-18）：3项独立原创服务测试覆盖同一受检socket后的认证、IPv4/IPv6、固定/分块/媒体、状态/帧/头/字节异常、截止/同步上下文、零请求拒绝和无重试/跳转，共享OneBot/媒体解析。受检WS见后项，托管CLI选择仍待接合，未调用真实QQ API。

受检WS见CHECK-013（2026-10-18）：2项原创独立服务测试核对Upgrade前原socket归属、IPv4/IPv6事件帧、错Job/服务/截止/凭据拒绝零请求；共用握手/帧，另1项超大握手测试验证SPEC-004的32KiB阶段预算。未接托管watch选择或完整HTTP/WS账号关联，不调用真实QQ API。

原生调度日志见CHECK-013（2026-10-18）：j1在fresh启动意图事务内先保存Job再创建根，未知/重复不重放，签名完整根身份及Job成员精确恢复；2项原创测试及完整180项回归通过。并发纠错证明进程退出与Job命名消失不保证同刻，固定截止/空成员观测及5轮复核记录于CHECK-013；不升级为真实QQ或完整启动门禁验收。

连接服务发现与事件Source见CHECK-013（2026-10-18）：既有TCP/独立HTTP-WS夹具扩展Job内服务PID发现、完整映像/原流匹配、零字节拒绝及失败替换丢弃旧连接，完整180项回归通过，不重复增加测试数量。持久服务绑定、客户端工厂及账号关联仍待接入，无真实QQ操作。

持久服务绑定见CHECK-013（2026-10-18）：j1服务身份/端点在t1成功后保存，重复不复制、缺旧日志拒绝修补，恢复核对run/安装摘要/端点/完整进程及Job；既有原创原生服务测试扩展和完整180项回归通过，不新增测试数量。完整客户端工厂、账号关联与真实QQ仍待验收。

客户端工厂接合见CHECK-013（2026-10-18）：普通/事件/doctor统一选择，受管记录禁止降级、Context实例/端点绑定、缓存回执离线及已有材料丢签名密钥不补建，完整183项回归通过。捕获自有Job中的conhost辅助成员，推翻“根退出后成员必空/仅原PID”断言，改为整个Job名称有界退出观测，不放宽生产门禁。旧合成登记迁移与受管CLI正向/账号关联仍待验收。

严格受管CLI见CHECK-013（2026-10-18）：新增独立CLI的签名恢复/status/联系人/账号变化及记录损坏缺失拒绝，精确13请求；旧send/reply/file/event夹具迁移原创Job内Rust服务，生产普通HTTP/WS回退删除。完整184项回归通过，53/86前阶段由54/87接续；原版启动、登录恢复与真实协议仍未验收。
