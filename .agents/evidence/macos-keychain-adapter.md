# macOS Keychain凭据适配

文档编号：CHECK-079。状态：Native adapter verified; cross-identity cleanup pending。日期：2026-10-24。
关联：SPEC-008、TASK-006。起点：048cdef。

## 实现与来源

采用Cargo锁定的security-framework 3.7.0安全API，目标限定macOS arm64。核对依赖的passwords.rs：创建失败为DuplicateItem时走SecItemUpdate，删除直接检查SecItemDelete结果；未使用忽略删除错误的上层封装。依赖原已用于TLS，本次只增加目标平台直接依赖，没有复制上游代码。
选非云同步条目，以现有规范状态目录命名空间存储。空键/控制字符在访问前拒绝；禁止交互期间串行操作，原先关闭交互时不创建会恢复为允许的guard。缺失与访问失败分离；没有明文后备或旧权威替换。

## 验证与真实边界

显式原生合成测试通过：唯一临时命名空间、缺失、创建、读取、更新、另一命名空间隔离、删除及重复删除；测试最终确认条目缺失。补充以同一可执行文件新进程读取并删除原进程条目成功；异常返回与正常操作后的Keychain交互设置均恢复。该测试默认ignore，需要显式执行，不把未执行算通过。
首次CLI安装集成测试能通过签名凭据初始化并到达来源不存在错误，但另一测试程序清理CLI创建的Keychain条目失败。完整门禁的首次重跑同样失败；两个条目使用随机临时状态命名空间，仅含合成签名密钥，没有QQ账号/聊天/实际凭据。临时目录被删除，未记录精确service，不能安全直接清理。元数据查询被自动审批拒绝，待用户授权，不将清理报告为完成。
普通CLI安装测试已改为预置合成签名材料并验证缺少原密钥时SIGNING_KEY_REQUIRED，避免再创建跨程序条目；未知版本拒绝仍先于状态初始化，实际源文件校验由已有安装测试覆盖。Windows原有行为保持。
scripts/check-macos.sh：fmt、Clippy、228项普通测试及arm64 release构建通过；显式原生Keychain测试另行通过。随后补充的同可执行文件新进程测试通过，并复核all-target Clippy。源码包258项白名单及完整源码覆盖检查通过；Windows原生回归未运行。Semifold变更集macos-keychain-adapter。不同签名程序、CLI升级访问身份、锁定Keychain、多实例并发、跨CLI签名恢复与私人目录加固均未因单程序测试而获得验收。
