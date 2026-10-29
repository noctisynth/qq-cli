# 账号资料及状态领域任务

文档编号：CHECK-140。状态：首个账号领域切片离线通过。日期：2026-10-29。
关联：RFC-0009、SPEC-009、TASK-007、CHECK-139。

固定NapCat v4.18.38/bbd0d2aca3d470c20b33b1d4f95123335633bf26的R01十一项处理器经源码核对；五项资料/状态载荷由实际Rust协议类型派生Schema。领域入口为account set/avatar/presence/status-text，分别支持昵称、签名（含显式清空）、性别、本地头像快照、在线状态及自定义状态文字。头像输入上限8MiB，预览不输出原文/路径/字节。

领域命令复用CHECK-139的受检实例、账号、权限和同键意图，返回严格领域回执；同一载荷经领域和api入口共用键，不能重复执行。set_qq_profile/set_self_longnick显式data.result非零即拒绝，不能仅凭外层retcode=0报告成功；错误省略上游任意原文。

上游get_online_clients恒返回空数组，_get_model_show返回固定值，_set_model_show为空处理器；describe与call标记compatibilityOnly，未将设备或机型业务计为实现。nc_get_user_status依赖Packet，点赞/用户状态领域任务另列待完成。

原创受管独立CLI验证六种路径（资料、签名、头像、在线、自定义状态、内部失败）、准确载荷、严格Schema、同键离线缓存及跨入口防重。离线预览与Schema测试覆盖空签名/空文字、缺键/缺字段/性别依赖拒绝。相关集成回归共30项通过（其中input_contracts/public_contracts六项单独通过，其余24项最终复核通过）。all-targets Clippy、Windows release构建、结构/契约检查、Skill quick_validate、源码包白名单440文件、fmt/diff通过。

未执行真实QQ操作或macOS新增命令真机验证；回执表明上游已接受，不替代客户端界面或实际状态的独立验证。R01仍部分完成，组内余项、其它组与完整交付继续由TASK-007追踪。
