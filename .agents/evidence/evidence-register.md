# 证据登记

文档编号：EVIDENCE-001。状态：Active。更新：2026-10-16。

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
