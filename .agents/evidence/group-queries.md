# 群资料与成员查询

文档编号：CHECK-144。状态：群查询领域切片离线通过。日期：2026-10-29。
关联：RFC-0009、SPEC-009、TASK-007、CHECK-141。

固定NapCat v4.18.38/bbd0d2aca3d470c20b33b1d4f95123335633bf26的R03七项处理器已核对，实际Rust载荷派生Schema及固定来源由api describe提供。额外核对同提交[GroupApi](https://github.com/NapNeko/NapCatQQ/blob/bbd0d2aca3d470c20b33b1d4f95123335633bf26/packages/napcat-core/apis/group.ts)、[群类型](https://github.com/NapNeko/NapCatQQ/blob/bbd0d2aca3d470c20b33b1d4f95123335633bf26/packages/napcat-core/types/group.ts)、[禁言通知类型](https://github.com/NapNeko/NapCatQQ/blob/bbd0d2aca3d470c20b33b1d4f95123335633bf26/packages/napcat-core/types/notify.ts)、[WebApi](https://github.com/NapNeko/NapCatQQ/blob/bbd0d2aca3d470c20b33b1d4f95123335633bf26/packages/napcat-core/apis/webapi.ts)及[OneBot构造映射](https://github.com/NapNeko/NapCatQQ/blob/bbd0d2aca3d470c20b33b1d4f95123335633bf26/packages/napcat-onebot/helper/data.ts)；搜索只定位文件，事实以固定提交为准。没有复制上游实现。

group member/detail/extended/share/honors/muted/mention-quota复用受检Session、前后账号/版本核对和目标群权限。成员核对group_id/user_id，扩展核对groupCode/resultCode；详细资料省略恒空备注和成员映射中的恒定装饰字段，扩展只投影已核对群主及策略字段。禁言原生字段为uin/nick/cardName/shutUpTime，不以示例中的user_id误认实际返回。列表与嵌套荣誉计入scanLimit，头像/分享URL不跟随。

两个原创测试覆盖七项参数Schema/只读元数据、缺目标/非法类型前置拒绝；受管独立CLI十四种场景检查载荷、严格响应Schema、缓存选择、全员禁言、群主、链接、荣耀与禁言空结果、跨群/成员不符、内层错误和超限。七种读任务在其它群目标下全部拒绝，零业务调用。CLI十四项、Skill三项及原群管理两项回归通过。all-targets Clippy、结构/契约、Skill quick_validate、源码包白名单460文件、fmt/diff通过。

未执行真实QQ/macOS新增链路验证。成员处理器依赖上游缓存已存在，CLI不自动预热重试。禁言监听失败、荣誉抓取失败折为空数组，strong_newbie为源废弃空结果，因此complete:false且不据空结果断言不存在；角色/时间/配额是后端快照，不能作为长期权限权威。其它组及完整验收继续由TASK-007维护。
