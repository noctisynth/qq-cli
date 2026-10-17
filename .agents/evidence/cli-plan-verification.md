# CLI 首版方案核对

文档编号：CHECK-007。状态：Documentation verified。更新：2026-10-17。
关联：PLAN-001、SPEC-004、RFC-0002、TASK-002。

本轮只完善用户文档和 Agent 上下文，没有 CLI 实现、依赖安装或新运行实验。
以 DATA-001 固定 Framework 产物读取 get_status、get_version_info、get_login_info、get_msg、
get_friend_msg_history、send_private_msg 的命令/schema/处理逻辑，结合 CHECK-005、CHECK-006 的有限实测。
核对历史查询的字符串用户 ID、count/disable_get_url/parse_mult_msg，及 get_msg 对消息映射的依赖；
明确 good=true 不是全面健康检查。没有复制上游实现作为 CLI 源文件。

建议工具链依据为 Node 官方当前发布表及 Node 24 文档；查阅日期为 2026-10-17，属于可变来源。
尚未核验实际编译器或依赖组合；运行时选型为推荐，未实施，不从已有 package.json 解析器反推选型。
直接抓取一个猜测的历史消息源码 URL 未命中，随后用固定 Git tree 确认 go-cqhttp 目录，
接口语义来自已校验产物，未将失效链接当作成功源码读取。

方案交叉检查：六个 action 的白名单、命令范围、账号门禁、JSON/退出码、发送未知结果和 M0 至 M4 验收相符。
检查文档编号、引用、仓库内 Markdown 链接、敏感标记、Git 空白及 Semifold 状态。
只为用户技术方案及 README/贡献指南创建 patch:chore；内部 Spec/RFC/TASK/证据不单独创建 changeset。
这些检查不是 CLI 构建、接口实现或真机回归测试；所有产品实现项仍未勾选。
收尾结果：29 个文档/数据编号唯一且引用完整，2 个仓库内 Markdown 链接存在；
公共文本隐私扫描、Git diff 空白及 Semifold 配置检查通过，固定审计 bundle 的 SHA-256 复核一致。
Semifold 当前仅有本轮用户文档对应的 1 个 patch:chore，未消费、提升版本或发布。
