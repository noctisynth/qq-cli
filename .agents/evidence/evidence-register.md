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
