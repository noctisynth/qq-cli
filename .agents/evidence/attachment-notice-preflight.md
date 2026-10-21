# 附件与通知功能补验准备

文档编号：CHECK-059。状态：离线准备通过，真实补验待执行。日期：2026-10-21。
关联：SPEC-004、TASK-001、TASK-002、CHECK-040、CHECK-048、CHECK-058。

用户确认登录方式由官方QQ与用户操作，不为扫码/密码/点击登录分别要求真实验收；CLI仍负责状态观察、实际身份确认及账号变化隔离。普通登录已有真实证据，不扩大为登录挑战或账号安全保证。

核对既有固定v4.18.38 OpenAPI共178条路径，无创建群聊接口；create_group_file_folder仅创建群文件夹。当前公开文档 https://napneko.github.io/onebot/api （2026-10-21检索，可变来源）亦未列出创建群聊，不能把未发现接口提升为QQ底层不存在能力。qq-cli没有建群命令；不新增加载器或以未审计内部接口尝试建群。

源码补核（同日）：git ls-remote固定当时HEAD为450fb0429a064e3418e0c698b1b4d78324101929。其packages/napcat-onebot/action/router.ts的ActionName无创建群聊动作，仍有create_group_file_folder、set_group_admin、set_group_leave及delete_msg。packages/napcat-core/services/NodeIKernelGroupService.ts第196/198/330行则声明createGroup(arg: unknown): void、createGroupWithMembers(arg: unknown): void、createGroupV2(arg1: unknown, arg2: unknown): unknown。这是QQ原生服务的未完整类型声明，不是已封装/注册的NapCat远程API，无法由声明推断可用参数、回调或运行兼容性。源码链接固定为https://github.com/NapNeko/NapCatQQ/blob/450fb0429a064e3418e0c698b1b4d78324101929/packages/napcat-core/services/NodeIKernelGroupService.ts 和同提交的packages/napcat-onebot/action/router.ts。

核对范围：GitHub文件树API因限流失败；完整源码包下载包含大量原生二进制，主动中止，仅扫描其中完整读取的329个文本文件，不能称全仓库全文检索完成。上述服务声明及API表随后从固定提交独立下载完整原文复核。结论限于公开API无建群入口、内部存在未完整定义的方法；不声称QQ无法建群，也未执行任何内部方法或真实群操作。

离线执行cargo test --offline --locked -j1 --test attachment_cache --test attachments --test media --test media_sources -- --test-threads=1：3+4+5+3共15项通过。覆盖缓存保留/清理、快照及签名所有权、媒体原子输出与字节/来源限制。首次受限沙箱在私有状态准备处STATE_IO，1项通过2项失败并提前结束；正常本地权限原样复核15项通过，未修改生产代码或降低ACL。既有增量缓存硬链接警告保留。

本阶段无QQ读取、发信、启动、停止、好友或群管理操作。用户已授权附件/通知测试及提出新建群；账号、群目标和具体测试操作方案已请求确认，确认前不执行依赖操作。新增好友与入退群不复用过往消息授权；真实格式及剩余通知仍未提升为验收通过，稳定性继续暂缓。
