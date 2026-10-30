# 普通好友申请查询与审批衔接

文档编号：CHECK-164。日期：2026-10-30。状态：Windows完整离线门禁通过，真实及macOS原生验收未完成。关联：RFC-0009、SPEC-009、TASK-007、CHECK-142。

CLI改动基于ad1666325a6d235002ee4e6f125100e06b973cc7的本地工作树；最终提交由Git记录对应本证据，不把基线提交写成已含新增能力。

## 来源与实现范围

源码观察基线为NapCat v4.18.38 / bbd0d2aca3d470c20b33b1d4f95123335633bf26。已核对本地原版napcat.mjs SHA-256为f79260cb7e6c4b579a4346ffd39a91a33ea73f9b7cd44d9ce204ab471096601a，未执行该产物。FriendApi.getBuddyReq使用getBuddyReq/onBuddyReqChange，SetFriendAddRequest按reqTime寻找申请，BuddyReqType中入站申请为1；OneBot既有监听器会清除未读计数。对应固定来源：[FriendApi](https://github.com/NapNeko/NapCatQQ/blob/bbd0d2aca3d470c20b33b1d4f95123335633bf26/packages/napcat-core/apis/friend.ts)、[审批处理器](https://github.com/NapNeko/NapCatQQ/blob/bbd0d2aca3d470c20b33b1d4f95123335633bf26/packages/napcat-onebot/action/user/SetFriendAddRequest.ts)。只记录原创归纳，不提交上游材料。

contact friend-requests输出待处理入站普通申请flag/uid/nickname/message，保留contact requests只查询可疑申请。原创friend_requests.cjs在同一已初始化session和现有受管OneBot HTTP端口增加固定查询路径，严格空对象载荷、精确Bearer、输入截止/预算、固定结果字段；未修改上游文件/插件限制、未新建监听端口。窗口历史范围不明确，complete:false；UID不能替代QQ号，受限targets时拒绝查询及裸flag审批。普通审批继续调用set_friend_add_request及共享写意图，不从申请内容推定操作授权。

Windows r1新准备模板2将原创加载器与查询模块作为核验材料并计入启动摘要；模板1保持原入口和字节。macOS将查询模块纳入原adapter发布摘要，Framework初始化后安装；当前主机未执行macOS原生构建或真实加载。

## 离线验证

- Node原创合成测试6项通过：筛选入站未处理普通申请（含已读）、排除主动/已处理/可疑/未知状态，准确flag、重复flag/无效字段拒绝、空列表、并发合并、无关联ID通道超时/抛错后禁用、原始错误不泄漏。
- HTTP回环夹具验证精确Bearer、POST、严格空对象及输入预算；其它路由保持原处理器。实际原创Windows入口加载临时原创ES模块，核对同一个session/login和初始化完成后才安装查询模块，没有加载QQ/NapCat。
- contact_tasks、runtime_config、launch和skills共11项通过。受管独立CLI合成链路从friend-requests输出读取flag，交给respond --approve true，并核对准确载荷、Schema、账号作用域、请求键防重和跨api入口恢复；受限目标明确拒绝。
- runtime_store新增定向测试通过：可执行适配字节篡改拒绝；重建模板1签名材料后恢复仍不安装适配器，原三配置文件和旧入口语义保留。
- 完整check.ps1通过：Node适配器合成、结构/单源契约、发布/安装器/版本锁合成检查、fmt、all-targets Clippy、全部串行Rust测试、Windows x86_64-pc-windows-msvc release构建及源码包白名单538文件。新增命令后已更新帮助目录计数至144个叶命令/164个帮助入口；最终Node 22.22.3下20项适配器测试补充复验通过，包含无关HTTP事件的原参数数量保全。
- semifold config sync --check及status只读检查通过，现有累计31个changeset规划0.2.0；未执行version/publish、消费changeset、推送或发布。

上述测试仅使用原创临时模块、合成账号/状态/凭据与回环端点。沙箱不允许回环连接，相关测试经自动审查批准在沙箱外运行。本机Windows未获创建macOS入口负例符号链接的权限，因此macos_entry的该项不计通过；其余入口夹具已运行，正式macOS门禁仍待原生环境。Windows门禁执行可跨平台的普通申请及bridge测试，macOS门禁还保留完整entry测试。

## 限制

发布门禁补充（2026-10-30）：[CI 41075148958](https://github.com/noctisynth/qq-cli/actions/runs/41075148958)暴露两处夹具期望遗漏。Windows临时目录的8.3短名与生产规范化长名不同，断言改为复用既有canonicalize期望助手，本地launch三项通过；macOS适配发布测试从三文件期望改为四文件，并逐字节核对新增原创查询模块，原私有权限与重复发布拒绝检查保留。生产路径、所有权与发布门禁未放宽；后续结果以修复提交的双平台CI为准。

未启动、登录、停止、注入、读取或审批任何真实QQ。原生服务是否接受该原创监听器、实际返回多少旧申请、与QQ窗口的一致性以及新增Mac适配均未真机验收。查询可能通过上游既有监听器清除申请未读计数；迟到原生回调不带查询ID，超时后整进程内拒绝后续查询，不自动重派。已运行的旧Windows实例无新增入口；本次开发没有改写其材料或自动重启。查询时重复flag会拒绝，但既有审批接口仍按上游当时快照查找，并非跨查询/审批的原子事务。
