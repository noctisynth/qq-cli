# 群撤回与管理员变更通知验收

文档编号：CHECK-061。状态：指定通知功能通过，两项观察待查。日期：2026-10-21。
关联：SPEC-004、TASK-001、TASK-002、CHECK-048、CHECK-059、CHECK-060。

范围沿用本轮用户明确授权：仅两个既有实验账号与指定群，合成附件、一条待撤回文字，群主临时设置另一实验账号为管理员并取消。不新增好友、不入退群、不新建或停止QQ；私人账号、群号、正文、回执、消息引用与运行输出仅在忽略目录。

qq-cli无撤回/管理员写命令；私人Rust测试触发程序复用原创OwnedHttp源码和既有签名run/Job/服务进程/实际socket归属检查，在发送凭据前核对原控制权。独立核对get_login_info的实际账号及群主/目标成员，固定三个允许操作并在每次变更前create_new持久尝试标记，不打印凭据，不开放通用产品API，不修改NapCat。写触发属于测试准备，不能冒充qq-cli提供群管理命令。事件读取均为发布CLI指定群event watch。

结果：实验成员经CLI发送一条合成文字，accepted且cached:false；一次delete_msg成功。两个账号均各收到一条supported group_recall，消息ID分别对应各自此前收到的该消息，user/operator匹配发送账号；跨账号局部messageId不同是预期作用域差异，不互相比较。各无重复或gap，正常deadline退出0。

一次set_group_admin enable:true成功，目标账号收到一次group_admin/set。私人测试脚本把成员列表立即更新误作为必需断言而失败；当时未重复设置。直接成员查询也仍返回旧role，首次恢复程序因此在发请求前拒绝、没有取消尝试标记。改为检查本次成功设置的持久回执后，重新建立只读监听，仅一次enable:false成功；目标账号收到一次group_admin/unset，目标一致。两次取消监听均deadline/退出0、零重复/gap。目标账号自身列表最终role=member，两账号status均原身份ready；不终止任何QQ。

角色新鲜度限制：群主侧列表最终仍role=admin，目标账号侧列表为member。直接在群主侧调用get_group_member_list/get_group_member_info，no_cache=false/true均返回admin；因此差异存在于NapCat原响应，非CLI序列化转换产生。不能因提供no_cache就保证服务器实时状态，也不以当前实验确认底层缓存失效根因。取消回执、目标unset通知与目标自身列表共同提供本次恢复证据，群主侧读取一致性保持待查。不在CLI伪造角色或合并事件覆盖后端值，不修改上游。

CHECK-060的单次event_error/BACKEND_PROTOCOL另行排查：两次有界、仅字段结构的受检只读观察均仅到正常截止，未复现错误。原触发帧未保留，无法确定是哪种事件及错误字段；不归因上游、不声称修复、不把成功重跑当故障已消失。保留原摘要和该独立功能待查项，后续需捕获脱敏触发结构后作确定性复现。通知本轮正常通过不关闭此项；稳定性专项仍暂缓。

离线：cargo test --offline --locked -j1 --test events --test event_schema --test watch -- --test-threads=1共6+1+6=13项通过。首次私人触发程序编译及列表形状/缓存断言失败均只属测试材料，已修正，不改生产Rust、不新增公开命令，不创建代码changeset。纯Agent上下文提交；业务混合计数68/100不变，不当完成比例。剩余通知入群/退群/好友新增仅合成验收，未获本轮授权，不执行。
