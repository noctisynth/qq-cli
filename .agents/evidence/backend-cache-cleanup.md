# 上游缓存清理与生命周期去重

文档编号：CHECK-163。状态：clean_cache领域任务/直接入口及离线门禁通过。日期：2026-10-30。
关联：RFC-0009、SPEC-009、TASK-007、CHECK-162/161。

来源固定NapCat v4.18.38/bbd0d2aca3d470c20b33b1d4f95123335633bf26/system/CleanCache.ts。空载荷处理器先unlink临时目录直属项，再处理当前账号nt_data中的Pic/Ptt/Video/File/log直属项；非递归，部分失败只写日志，void返回无删除计数或完整清理证明。按用户续订，bot_exit/set_restart不新增专用封装/载荷Schema，停止重启需求沿现有napcat stop/restart的签名生命周期；通用动态入口仍保留。

napcat clean-cache --request-id KEY及api call clean_cache共享api.clean_cache意图、账号身份与HMAC指纹，缓存跨入口不重发。写前pending、实际归属/账号核对、一次请求、后置身份；send拒绝/targets受限为零业务请求。原生null或缺data是void应答，可接受但cleanupVerified始终false；原生失败/异步/异常对象/身份变化均unknown，不把可能部分清理记成安全重试。裸流原生清理错误也unknown。无CLI文件删除、计数补造、自动重启或media prune代偿，dry-run不读配置/凭据或连服务。

原创tests/backend_cache.rs两测试、12种受管模式通过：正常/缺data、原生失败/async/异常对象、身份改变/派发前身份拒绝、send/target限制、raw首调/后缀/伪造目标、同键及跨入口缓存；本地合成标记文件始终保留。CLI14、Skill3、R16工具2（21模式）、streams2（新增清理错误unknown）和共享API13单测回归通过。Skill长度门禁及导出链接先发现超预算/片段路径问题，已精简/修正并重跑通过，未放宽门禁；Windows/runtime与macOS指引均低于9000字节，11任务/14导出文件保持。

最终all-targets clippy -D warnings、fmt、架构/契约、源码包530文件、Skill quick_validate及diff通过。最终release构建通过，SHA256 b317c60aa5bdf17f6bac67d881c91f12c7e35954b7f67faa5a6ec8fc60f1231f。离线release冒烟核对143叶命令、12个R16/clean_cache接口描述/Schema/结果说明、exit/restart未新增Schema、凭据敏感标记/URL兼容标记、清理dry-run/help和技能预算；真实QQ操作0。没有实际清理QQ缓存/读取真实凭据/发真实数据包，真实效果与macOS沙箱/清理范围另验。未重跑整个原393项累计套件，本轮针对受影响用例执行上述回归；历史全套范围仍见CHECK-161。

R16提交2aa3b0d及backend-tools changeset已完成，本切片另有backend-cache-cleanup changeset。semifold status共26项、规划0.2.0，当前包版本仍0.1.1；没有消费变更集、推送、安装或发布。
