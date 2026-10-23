# Framework 文档历史、弃用范围与账号风险

文档编号：FACT-004。状态：History-and-static-verified。更新：2026-10-24；静态调查截止：2026-10-16。
关联：FACT-002、FACT-003、EVIDENCE-001、CHECK-004。本次静态调查未运行 QQ 或上游产物。

## 结论与修正

原生入口并非只有附件存在这一条证据：维护者曾在官方 issue 明确推荐 napiLoader 实现有头加载。
明确停止维护的是旧 Framework Once/OneKey，不是这些答复中的原生替代入口。
当前指南只写 LiteLoader；未找到为何未补写原生入口的维护者解释，也未找到后来正式弃用原生入口的声明。
历史推荐不能证明当前版本兼容、长期维护承诺或账号安全；FACT-003 的生成链与支持矩阵缺口仍成立。

本方案不能保证账号不被下线、限制或封禁。不依赖 LiteLoader 只改变加载依赖，
没有消除非官方进程内接入、Hook、检测和平台处置风险，也没有取得 QQ 官方授权。
如必须保证这些风险不存在，当前 NapCat 候选不满足此要求，不能作为已收敛的日用方案。

## 官方文档历史

文档仓库观察提交为 3cf609b5bc96d56df1dcf9cb3388a6b90665febf（2026-10-12）。
读取 Framework.md 当前路径可达历史的 20 次变更及完整 diff；未见 napiLoader、napimain、napiloader 或 nativeLoader 说明。
全 src 的跨历史搜索因按需对象下载耗时而中止，不把它宣称为完整全站历史检查。

| 时间 | 固定提交 | 实际修改与边界 |
|---|---|---|
| 2025-05-12 | c13e2b9bed8add13d816c69049affaf425b06377 | 增加 QQ 9.9.19 后 LiteLoader 维护欠缺、鼓励迁移 Shell 的警告；不是原生入口弃用说明 |
| 2025-10-14 | d1c261af78d53e57bd594ed483392a5a3bf08ace | 删除旧 Windows Installer 章节；发生在目标原生文件引入之前，不能当其弃用证据 |
| 2026-10-10 | 5f07655fc59b24f58b8adad79699d998a0a34619 | 简化安装指南，删除 Once、绿色版、旧安装脚本和维护警告，仅保留 LiteLoader 导入方式；没有删除 napiLoader 章节，也没解释其缺席 |

当前文档仍列 LiteLoader 不等于它重新获得维护或官方账号安全保证。
不使用“文档过时”或“原生入口被弃用”填补未解释的差异。

## 官方 issue 的直接答复

| 来源 | 维护者答复时间 | 可支持结论 |
|---|---|---|
| NapCatQQ #1375 | 2025-11-10 | 放弃 Framework Once/OneKey 维护，指向手动下载 Framework.zip、使用 napiloader.bat 和已安装 QQ |
| NapCatQQ #1398 | 2025-12-05 | Once 因 LiteLoader 支持问题不再支持，指向 Framework napiLoader 实现有头加载 |

两项都是仓库 Member 的直接回复，强于下游运行报告；没有固定本轮 DLL 摘要或当前 QQ 支持矩阵。
问题关闭不意味着原生替代路线关闭。全局 GitHub 搜索仅辅助定位；搜索 API 对 napiloader 命中这两项、
napimain 未命中，文档仓库 Framework 命中一个 PR 和一个错字问题。后续一项查询触发限流，
不能把检索结果当作所有讨论的穷举。未联系维护者。

## Bypass 不是 Shell 独有

固定发布源码和 Framework ZIP 均包含 Framework 的 enableAllBypasses 调用。
Framework 在 NCoreInitFramework 内加载 Napi2NativeLoader 后调用；Shell 在初始化期间也调用。
调用由 NAPCAT_DISABLE_BYPASS 条件控制，并传入配置 bypass 或空对象；使用可选调用语法，
不能仅凭调用点存在就断言该平台二进制实现了导出、所有功能默认生效或调用成功。
配置及类型列出 hook、window、module、process、container、js；具体原生行为不能仅由字段名推断。

官方提交 62c92463681365feeffe634ec7d55eb3dbcf5f50（2026-02-13）的主题直接说明为反检测准备；
5c72f771c37273c5c3e91a655b777ae991818a7f 同日说明在 Framework 和 Shell 初始化中调用该导出。
b9f61cc0eee02465e9a69b3d99675c304265af35（2026-02-18）说明增加可配置反检测和因 Bypass 崩溃的恢复策略。
因此反检测意图有上游说明，具体拦截实现、覆盖范围和有效性仍未由本轮原生源码或运行核验确认。

必须区分启动/文件完整性校验、运行环境或 Hook 检测、服务端账号与行为风控。
不同层的处理不能互相替代；成功通过本地检查不等于服务端认可或不会封号。
禁用 Bypass 也不会撤销入口注入、会话 Hook 和其他初始化行为，不自动变成安全模式。
napi2native 原生能力组件与 napiloader.dll 是不同组件，不混用其来源或维护结论。

## 安全证据与产品影响

LiteLoader 官方仓库在 2026-05-17 归档，README 明确警告下线及封号可能性。
NapCat 自己的安全页也讨论掉线、社交风控及常用账号可能受影响，未承诺免封。
该页面包含历史配置建议，本项目不据此直接改动当前配置或量化风险概率。
维护者 Discussion #517 描述过应对 QQ 改动、Hook 和反检测能力，只作为历史架构背景。

API 鉴权、回环监听和命令白名单只能约束本地接口访问与误操作，不能保证 QQ 账号安全。
应分别记录本地数据/API 安全、原生稳定性、账号与平台风险；静态摘要验证只确认文件身份。
不得以“官方客户端”“只读”“低频”“禁用 Bypass”或“保留窗口”宣称消除风控。
上述结论限固定来源的静态调查；后续运行证据单独登记于 EVIDENCE-001，操作授权遵循当前用户指令及 [AGENTS.md](../../AGENTS.md)。
