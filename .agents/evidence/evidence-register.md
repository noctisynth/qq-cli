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
