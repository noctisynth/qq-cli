# 凭据与底层原生工具

文档编号：CHECK-162。状态：R16十一项协议/工具入口及离线门禁通过；URL检查仍为上游占位。日期：2026-10-30。
关联：RFC-0009、SPEC-009、TASK-007、CHECK-161。

固定NapCat v4.18.38/bbd0d2aca3d470c20b33b1d4f95123335633bf26：user/GetCookies、system/GetCSRF/GetCredentials、extends/GetClientkey/GetRkey/GetRobotUinRange/SendPacket、packet/GetRkeyEx/GetRkeyServer/GetPacketStatus、go-cqhttp/GoCQHTTPCheckUrlSafely及core/apis/user、OneBotAction已核对。Cookie/CSRF路径主动调用票据服务与QQ登录网络，三RKey源同一Packet操作，status处理器为空但前置检查packetStatus，URL处理器直接返回level1没有实际检查。保留原生响应，不把示例任意记录当真实字段结构，不伪造安全结论或详细Packet状态。

R16统一由显式api call承担纯工具用户链路，十read/一write及实际载荷Schema完成；api describe派生sensitiveResult/resultNote，URL兼容标记也进入调用结果。七项账号凭据/RKey在targets受限拒绝，任意packet不能靠附加user/group字段证明作用域；三全局工具可在受限目标读取。send_packet需read/send及原键；原生失败可能发生于发包后，记unknown并缓存不重发，裸流入口同样要求read。读操作只做一次显式业务查询；有键读取缓存状态，不重取秘密；无键不创建请求记录。没有扩大全局动态DTO，秘密/原文/数据包只在API Document响应边界。

原创tests/backend_tools.rs两测试覆盖十一描述与派生载荷、七项敏感标记/未知action、URL兼容与Packet空结果、21种受管模式：全部工具、原生值/未审阅记录保真、读写/目标策略、同键缓存、账号变化、原生拒绝与异步、只读有键仍无需send、原始域名/packet字节/凭据不落ledger。通过。API3/API CLI1、CLI14、Skill3、streams2回归通过（流式新增send_packet零业务请求read拒绝）。check/clippy全targets、fmt、架构/契约/源码包527文件及Skill quick_validate/diff通过。未读取真实QQ凭据或发真实数据包；上游服务/原生依赖与macOS另验。changeset backend-tools已创建，规划0.2.0，未推送或发布。
