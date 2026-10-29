# 联系人资料与好友关系

文档编号：CHECK-142。状态：联系人领域切片离线通过。日期：2026-10-29。
关联：RFC-0009、SPEC-009、TASK-007、CHECK-139。

固定NapCat v4.18.38/bbd0d2aca3d470c20b33b1d4f95123335633bf26的R02八项处理器核对完成，api describe链接固定处理器并从实际Rust载荷派生Schema。可疑申请返回字段进一步核对同提交[FriendApi](https://github.com/NapNeko/NapCatQQ/blob/bbd0d2aca3d470c20b33b1d4f95123335633bf26/packages/napcat-core/apis/friend.ts)。没有复制上游实现。

contact profile/categories/unilateral/requests提供严格有界投影；陌生资料核对请求用户，嵌套列表计入scanLimit，未知扩展字段不公开。集合按允许的private目标过滤；未知用户的可疑申请在受限策略下不暴露；输出complete:false，单向源仅请求99条，可疑count不保证账号全量。profile省略上游失效login_days字段。
remark/remove/respond/accept-doubt复用同账号/请求键意图，不保存正文、flag或上游原文。remove的valid:false或原生result非零持久记拒绝，缓存不会改报成功或重发。普通申请显式approve，疑似申请源实现忽略approve且只有接受，领域未提供虚假拒绝。flag无法证明目标时受限策略拒绝。

两个原创联系人集成测试验证八项参数Schema、四种写入离线预览、缺键/缺决定/非法值；受管独立CLI覆盖十三种场景（四种读取、七种写入及拒绝、身份不符、嵌套超限），核对准确载荷、目标过滤、严格响应Schema、缓存/跨api入口防重和错误原文不泄漏。CLI十四项及Skill三项回归通过。all-targets Clippy、结构/契约、Skill quick_validate、源码包白名单456文件及fmt/diff通过。app/tasks抽取为已实际出现的领域/直接调用路由，不引入新的运行权威或unsafe。

未执行真实QQ或macOS新增链路验证。普通申请flag的新增事件投影仍由事件专题跟进；上游复合操作的失败不保证此前子操作未发生，回执不是事务回滚承诺。其它组、扩大后的故障矩阵与完整验收继续保留。
