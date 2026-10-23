# macOS Framework 手动预加载实验

文档编号：CHECK-070。状态：真实启动尝试完成，Framework 预加载失败。日期：2026-10-23。
关联：FACT-005、RFC-0008、TASK-006。用户本轮明确要求手动用 Framework 启动 QQ；授权不扩展为发信、聊天查询、自动登录或终止其它会话。

## 元组与操作

- 本机 macOS 报告版本 27.0，宿主 arm64；QQ 为 Mac App Store 版 7.0.2-53644，package.json 的 isMas/isAppStore 均为 true，应用及 QQNT.framework 含 arm64 slice。
- NapCat 为 CHECK-068 固定 v4.18.38 Framework 原版 ZIP，解包前再次核对 SHA-256；安全检查路径和符号链接后解包到忽略实验目录，未修改上游文件。
- 启动前进程映像检查无运行中的 QQ。创建独立 NAPCAT_WORKDIR，WebUI 仅绑定 127.0.0.1:16099，随机 token 保存在私有配置，未输出凭据；不配置自动登录账号。
- 以 arch -arm64 启动官方 QQ 可执行文件，为该进程设置 NODE_OPTIONS 的 --require 指向原版 napcat.cjs。未创建自研加载器、未安装 LiteLoader、未改 QQ package.json 或签名。该环境预加载是待验证候选，不称为官方已支持的启动方式。
- 此独立目录仅隔离 NapCat 写入，不隔离 QQ 自身用户数据；未声称新进程等于新账号或数据隔离。未进行业务读取或发送。

## 观察

1. QQ 主进程和三个 Helper 进程出现，随后仍在运行；未自动终止本次 QQ。
2. 日志明确包含 Electron node_bindings 错误：`Most NODE_OPTIONs are not supported in packaged apps. See documentation for more details.`
3. QQ 自己的日志出现 major.node/wrapper.node 的 preload 成功记录；这属于 QQ 自身预加载，不能混同 NapCat 成功。NapCat 关键字日志计数为 0，无 Framework 初始化成功记录。
4. 独立实验目录未生成 NapCat 运行日志；在正常宿主权限下连接配置的回环 WebUI 端口失败，connect_ex 返回 61（连接拒绝）。
5. QQ package.json 启动前后 SHA-256 相同。未修改 QQ 应用文件；不把这一项扩大成所有 QQ 用户数据未变。
6. Computer Use 获取 QQ 窗口失败，系统 ScreenCaptureKit 返回 -3811；应用清单仅确认 QQ 正在运行。未取得可见官方窗口或登录状态证据，不能称窗口/账号共存验收通过。

私人完整日志、启动记录与 result.json 留在忽略实验目录。复核时日志 SHA-256：`8c3eeac324579bf0d7b42f0248baeb42cdaabb456c1b85e27a17ccef25b5f24f`；进程仍运行，之后日志可能继续增长，此摘要仅标识当时快照，不作为整个未来文件的摘要。

## 结论与下一步

本次原版 CJS 环境预加载路径在受测 Mac App Store QQ 上失败。实际错误与零初始化/端口拒绝相互支持；不能由此推断所有 Mac QQ 发行版、所有入口或 Framework 核心均不可用，也不能宣布完整 Mac 接入成功。

首要缺口仍是能在 QQ 内核加载时序内执行 Framework 的有效原版入口。下一步可核对官网发行版与商店版的启动入口差异及上游提供的接入机制；直接复制同一 NODE_OPTIONS 命令不能视为新证据。改写应用入口、引入额外加载器或替换发行版必须先明确具体方案及其与既有产品约束的关系，不能将本次失败升级为自研加载器授权。共享窗口、实际身份、OneBot API、媒体和事件均未验收。
