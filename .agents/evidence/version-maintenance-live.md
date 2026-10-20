# 版本维护功能验收

文档编号：CHECK-043。状态：固定版本切换/回滚正常流程通过。更新：2026-10-20。
关联：SPEC-004、RFC-0003、TASK-002、CHECK-025、CHECK-042。基线cb4dae9，用户本轮明确授权版本维护/注销/Ctrl+C，范围为原实验实例；不新增发信。

v4.18.37官方固定源下载返回ARTIFACT_SOURCE_UNAVAILABLE，随后使用原已审计ZIP经公开napcat install --version VERSION --from ARCHIVE完整校验导入，cached:false。固定来源/摘要与CHECK-025一致，不混合两版文件。

默认update到缺精确实证的37在停止前COMPATIBILITY_UNKNOWN，原运行保持。补--verify-compatibility与--timeout：仅复用inspect_trial对完整Key构建本次AuthorizedStartupTrial，后续同锁检查及每次输入重核继续执行；不生成supported。首个标签不匹配导致同样停止前拒绝，改为门禁认可的原试验标签后执行。38→37返回AUTH_REQUIRED，按status核对实际37在线且账号等于原实验账号，不重发启动。

发现session版本允许列表仅38，导致37受检API返回VERSION_UNSUPPORTED。CHECK-025证明两版napcat.cjs相同、配置已双版本核对；补明确37/38身份协议允许列表，不从安装目录泛化未来版本，新增精确匹配/未知拒绝测试。源码改动不涉及NapCat。

调试构建rollback先后在30/120秒预检截止，登记持续为原37 ready/run且未停止；本地精确37实证已按原严格观察产生。补rollback --timeout，最终正常release以120秒执行一次签名目标回滚38，返回AUTH_REQUIRED后status确认38在线/原账号。same-target update返回reused:true；doctor.canStart:true，两个精确本地记录。另9个原QQ进程ID和创建时间不变。阶段性登录检查点不当作启动失败重放；回滚目标来自原签名历史而非版本排序。

系统侧：一个此前release构建在使用期间被Windows防护标为Trojan:Win32/Bearfoos.B!ml并隔离，分类原因未定，不宣称误报或安全。未恢复隔离、未排除或修改防护。后续对实际修复代码正常release构建成功且用于本轮回滚/status/复用；此不消除发布兼容/安全审核事项。PowerShell重定向曾未能创建进程、Python首次启动路径错误，均未执行维护，不混同QQ故障。

验证：CLI12、startup6、runtime_store20、runtime_stop10、版本门禁新增1、owned_transports8及status_login3，共60项唯一用例定向通过；首次非提升沙箱的CLI夹具STATE_IO失败保留，原生环境复核通过。fmt、all-target clippy、正常release、架构及217文件源码包通过，Semifold未消费。调试构建预检耗时保留，不宣称性能验收。真实版本维护不等于自动目标准备、任意版本兼容或故障专项通过；67/100原混合计数保持。

私人命令与完整输出/状态/进程证据只留忽略目录；公开记录不含账号/路径/凭据。下一步Ctrl+C，再停止后注销，稳定性专项暂缓。
