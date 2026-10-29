# 群申请与通知用户链路

文档编号：CHECK-145。状态：群申请切片离线通过。日期：2026-10-29。
关联：RFC-0009、SPEC-009、TASK-007、CHECK-139。

固定NapCat v4.18.38/bbd0d2aca3d470c20b33b1d4f95123335633bf26的R05四项处理器逐项核对；api describe提供固定来源和实际Rust载荷派生Schema。group requests的system/legacy-notices/doubt分别读取普通可调count、普通固定50条及可疑type=7固定10条；后两项上游名称虽含ignored，源码不等于按忽略状态过滤。结果去除邀请兼容字段重复项，按群/目标策略过滤，本地limit不扩张上游批次，complete:false并保留来源/截断信息。

源处理器把seq字符串转为Number，故observedRequestId仅是观察；只有正JS安全整数才形成requestFlag，超界不拼造可用flag。group respond显式approve/reason/search-limit，使用所选实际账号和共享意图，不回显/持久flag或原因。受限目标下纯flag不能证明群而拒绝，未增加被上游忽略的group_id“证明”。

两个原创测试覆盖四项载荷Schema（含reason:null）、离线预览、缺键/缺决定/非法来源与预算；受管独立CLI七种场景检查三源载荷、群过滤、别名去重、本地截断、超安全整数无flag、接受/拒绝、上游拒绝及超限，成功/失败均缓存并与api入口共用键。受限flag审批与其它群过滤请求零业务调用拒绝。首版超界数字夹具缺Rust类型后缀，修为u64后通过。CLI十四项、其它群四项及Skill三项回归通过；all-targets Clippy、结构/契约、Skill quick_validate、源码包白名单464文件、fmt/diff通过。

未执行真实QQ或macOS新增任务验证；原始事件flag与类型的完整来源仍由事件专题实现。列表批次和观察ID不是全量/权限或审批授权，已接受回执不替代最终群成员状态证明。其它组及整体验收继续由TASK-007追踪。
