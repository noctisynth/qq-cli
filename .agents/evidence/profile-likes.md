# 用户点赞及在线状态

文档编号：CHECK-143。状态：领域切片离线通过。日期：2026-10-29。
关联：RFC-0009、SPEC-009、TASK-007、CHECK-140、CHECK-142。

固定NapCat v4.18.38/bbd0d2aca3d470c20b33b1d4f95123335633bf26的SendLike/GetUserStatus/GetProfileLike处理器源码为协议依据。contact like USER --times N为显式一次写入，复用共享意图；contact status USER投影原生status/ext_status，保留Packet依赖。contact likes省略用户查自己，查询他人须从同账号资料取得expected-uid，返回UID不符即SUBJECT_MISMATCH，阻止源实现无法解析目标时回退自己而产生误归属；没有隐式额外资料查询。

点赞结果只投影已核对计数/时间/UID及可解析的QQ号，不伪造未解析的uin=0；start/limit明确批次，complete:false。R01剩余六项参数Schema加入离线详情，十一项均以实际序列化类型派生；三个设备/机型上游占位仍标compatibilityOnly，未声明真实设备业务完成。

原创受管CLI用例在CHECK-142场景上增加在线状态、点赞、防重、自己/他人查询及UID回退拒绝；准确载荷、参数/响应Schema、未知字段不泄漏与缺UID/非法次数前置拒绝覆盖。扩展后的十八种场景通过；相关API三项、CLI十四项、Skill三项回归通过。all-targets Clippy、结构/契约、Skill quick_validate、源码包白名单456文件、fmt/diff通过。

未执行真实QQ操作或macOS新增任务实测。R01的实际设备查询/机型修改受固定上游占位限制，真实能力及其它组仍未因此完成；普通API存在不代表领域或完整验收完成。
