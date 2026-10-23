# Apple Silicon Framework 共享会话验证

文档编号：CHECK-072。状态：原型启动、共享身份及限定读取通过；非 CLI 平台交付验收。日期：2026-10-24。
关联：FACT-005、RFC-0008、SPEC-008、TASK-006。此为私人实验的脱敏摘要，不含账号、聊天、凭据、设备标识或本机路径。

## 固定基线与来源

| 项目 | 固定值 |
|---|---|
| QQ | 腾讯官网 DMG，7.0.2-53644，原生 arm64 |
| QQ DMG SHA-256 | d270b78f0b210532637076532a883dc8659851b6daa92451983985ab9f3216f0 |
| NapCat | v4.18.41，提交 859eb530c5801577633c2c3ae871d23586edf281 |
| Framework ZIP SHA-256 | 3c526f5789ff6cc56d78381eb10536c8273fe394d227187b27dea94b59965822 |
| Shell ZIP SHA-256 | 6801f93e533846cea05f4f531874a0e84371ab2331c5585b57cd7e3b869cf96b |
| 宿主 | 实测 macOS 27.0，Apple Silicon；不据此推定最低系统版本 |
| Mac 安装器观察提交 | accafd630b55656035a24486f56b3c08cb0505d1 |

来源：固定 [NapCat 提交](https://github.com/NapNeko/NapCatQQ/tree/859eb530c5801577633c2c3ae871d23586edf281)、
[发布页](https://github.com/NapNeko/NapCatQQ/releases/tag/v4.18.41)（可变元数据，2026-10-23 检索）、
[Mac 安装器](https://github.com/NapNeko/NapCat-Mac-Installer/tree/accafd630b55656035a24486f56b3c08cb0505d1)。
本轮检查的上游 main 为 3420f0b76d7dc0ac92cde1663f21ea27f61e13bb，仅删除一个测试文件，不将未来 main 视为同一基线。
两发布包 CRC/摘要核验；实验后 680 个 Framework 原文件逐个与发布 ZIP 比对一致。

## 操作与证据边界

用户先要求寻找不改变签名的入口；有界上游检索未确认可复用方案，不能据此证明全网不存在。
随后明确允许最小外部胶水、专用副本 ad-hoc 重签名、隔离启动、联网、手动登录及联系人/聊天读取。
不发送消息、不修改联系人或群，不改原安装、Gatekeeper/SIP；只清理实验自己的进程组。
原始日志、身份证明、完整请求结果及配置均留本地忽略目录，未复制到本记录。

## 过程与失败定位

| 阶段 | 观察及处理 |
|---|---|
| 原入口研究 | packaged app 不接受预期 NODE_OPTIONS 预加载；其它检查未取得可用保留完整原签名入口。CHECK-070/071 的失败仅适用于各自实验 |
| 专用副本入口 | package.json 入口改写破坏原资源封印；用户批准后仅重签实验副本 |
| 原版 CJS | 当前 Mac 会话类无 create；直接 napcat.cjs 失败。改为原版 nativeLoader.cjs 接收 QQ 自己取得的对象 |
| 原生 hook | 初始签名下触发 CODESIGNING Invalid Page；o3HookMode=1 未修复。主进程增加 unsigned executable memory 权益后存活，恢复 o3HookMode=0 |
| 方法观察 | 原生 init/getter 不可写；使用外部导出转发视图保留原行为，不改原生模块或 NapCat |
| 在线第一次 | 真实初始化通知到达；原生数据路径为空，NapCat 回退到原用户目录，sandbox 正确拒绝。增加空值路径兼容 |
| 在线第二次 | 初始化通知到达，但 QQ 文档组件推导 Group Containers 到实验根外；OneBot 默认配置名也写错。纠正隔离目录布局与 onebot11 配置 |
| 在线第三次 | WebUI 可连接，窗口正常；等待手动登录超时，不能计为会话或路径修正后通过 |
| 在线第四次 | 用户手动登录后真实通知、账号匹配、在线及限定读取通过；用户反馈窗口看起来正常，未再出现原存储权限错误 |

一次 native UI 观察工具超时后产生了额外实验副本进程；入口环境保护阻止其进入原 QQ JS，按精确副本路径清理。
后续不用该工具启动/观察，改为实验内 Electron 窗口状态；在线阶段不截图聊天。

## 验证方法与结果

- 离线 10 项适配测试覆盖真实通知前不释放、早/晚注册、严格布尔、过期代次、只读原生属性、冻结回调、receiver/返回/异常、原对象转交、空路径兼容及隐私字段白名单。
- 离线窗口在原生模块初始化后可见且未崩溃；在线第四次在约 12 秒观察到就绪及身份通过。
- 仅从官方 init 观察身份，用每轮随机盐的 HMAC 与 OneBot get_login_info 返回身份比较，代次一致；不保存到公共文档。
- onOpentelemetryInit 的 is_init 为 true，Framework 初始回调已释放；get_status 返回 online/good 为 true。
- 下表接口都返回成功；群详情目标与已选群一致，get_msg 与已读取消息对应。

| 类别 | 通过的实际接口 |
|---|---|
| 身份与状态 | get_login_info、get_status、get_version_info |
| 联系人和会话 | get_friend_list、get_group_list、get_recent_contact |
| 群资料 | get_group_info、get_group_member_list |
| 消息 | get_group_msg_history、get_msg |

私聊历史因最近会话快照没有私聊而跳过，不能记为通过。群历史仅有限样本，不证明分页或全量覆盖。
补查无 token 请求返回 403；监听检查确认 HTTP/WebUI 属于专用 QQ 进程且业务端口仅回环。
这不是产品级同 socket 归属竞态验收；实验未调用 Rust CLI 的完整 macOS 运行管理。
检查时日志未见 EPERM/EACCES/TypeError/unhandledRejection 或 Framework 初始化错误。
用户要求重开时仅一次将窗口带到前台；不据此声称已验证长期后台不抢焦点。

## 完整性与结论

原安装主程序、入口配置和资源封印摘要与实验前一致；原安装及实验副本的完整签名校验均通过。
副本使用本地 ad-hoc 身份，不宣称仍保留腾讯签名身份。Framework 680 个文件未修改。

此证据支持专用副本、最小外部适配、原版 Framework 的 macOS 开发方向，并取代“尚无可用接入路径”的当前判断。
它不否定旧失败观察，也不证明存在无需重签名的方案。
不涵盖写入、媒体、WS/恢复、账号切换、多实例、更新兼容、长期稳定性或账号安全；产品差距见 TASK-006。
