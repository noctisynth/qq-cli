# 证据登记

文档编号：EVIDENCE-001。状态：Active。更新：2026-10-16。

公开材料检索日期为 2026-10-16。以下链接均为可变分支，记录的是研究入口，未登记为固定版本实验。

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

SRC-002 与实际发布的启动器二进制尚未做一致性核对。后续需登记提交 SHA、上游许可、
发布标签、附件摘要和可复现步骤；不得以当前登记冒充已完成下载校验或同会话实验。
初始化验证见 CHECK-001；本登记不保存聊天内容、账号或本机安装目录。
