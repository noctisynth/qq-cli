# 本地启动实证与实例生命周期正常对照

文档编号：CHECK-042。状态：本地实证/安装恢复及重启/停止后启动通过，版本维护等仍待。更新：2026-10-20。
关联：CHECK-041、SPEC-004、RFC-0003、TASK-002。基线697ad8e，Framework v4.18.38、QQ 9.9.36-53644，原签名实验实例；用户明确授权继续日志保全、记录机制和生命周期。无发信/新账号实例或其它QQ操作。

恢复安装：按原签名清单及目录边界核对额外Tencent树仅含1个普通.log、无reparse；在忽略备份目录先复制快照，再完整移动该树，不删除字节。doctor安装检查恢复ok，QQ仍在线。日志内容及真实来源/备份路径仅留忽略目录，不纳入公共夹具。

新增compatibility/local有界k1签名索引，最多64条、按完整Key指纹读取，签名/结构/摘要冲突及未知成员拒绝，原密钥缺失不补建。显式生命周期可对ready原实例采集实证：先验证安装/QQ/主机Key；前后受检API在线账号/版本一致；状态事务内恢复原r1计划与秘密，重建原白名单环境/三参数/CWD/配置摘要，匹配原j1的完整input_hash；重开原服务Identity与原Job核对映像和成员后再发布。本地FrameworkStartup只对完整相同Key有效，不是上游支持/安全或所有消息功能保证。doctor只读装载，显式首次试验不直接升格。

dispatch抽取同一v1编码函数供分派与观察共用。旧安装CWD与新run CWD仅作两种明确候选，实际必须与旧签名摘要完全相同，旧j1不改写。本次旧原运行成功匹配并产生1条实证，doctor变为supported/canStart:true，证明不是按API或版本号直接伪造支持。

真实restart只执行一次：首次返回START_OUTCOME_UNKNOWN，按下一步只读status/doctor、检查原后继，不重复restart。原实例新run确有QQ窗口及进程，最终status恢复ready/online、账号与原实验相同，不需人工登录。随后stop返回stopped，status确认停止，再只执行一次start --timeout 120；返回AUTH_REQUIRED检查点后，status确认自动恢复相同账号ready/online，未因检查点重发启动。最终start复用返回reused:true/ready，doctor完整性ok、canStart:true。QQ日志位于当前run，固定安装未再出现Tencent树。

多候选无--instance的status返回INSTANCE_AMBIGUOUS，不任意选取。原QQ进程快照中另9个进程PID和创建时间保持不变；原签名实验组正常替换，未用taskkill或枚举PID终止他组。3个登记实例和原安装保持，未注销原实验实例。

验证：本地签名索引2项、pending/缺服务不得升格1项；startup 6、runtime_store 20、runtime_stop 10项通过；credentials 1、diagnostics 4、owned_transports 8、request 6及Skill 2项补充通过，共60项定向，不冒充完整套件。最终回滚无历史拒绝用例复跑通过（不重复计数），保持先选择目标再观察。clippy已修复一处needless_borrow后通过，fmt/release/架构/217文件源码包、diff/隐私与Semifold通过；重建的内嵌Skill内容核对通过。真实doctor前后注册表/引用文件/证据索引摘要完全一致，验证只读。原生缓存硬链接和既存Cargo许可元数据警告保留。日志移动后的展示脚本曾误用result字段报KeyError，doctor原结果实际成功，未重做迁移。未开展新稳定性专项。

范围限制：本次功能通过不等于remove/uninstall、真实多账号新建、版本37/38切换/回滚、Ctrl+C或全部事件通过。后续依功能优先继续，未知写结果不重放。生命周期大项仍含未验收范围，不整项勾选；原67/100混合计数保持，新功能子项单列。公开代码/用户文档有changeset，纯Agent证据无独立changeset。
