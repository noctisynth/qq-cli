# 空间发布删除与频道兼容占位

文档编号：CHECK-155。状态：R15空间领域链路离线通过；频道原生业务仍缺失。日期：2026-10-30。
关联：RFC-0009、SPEC-009、TASK-007、CHECK-154。

固定NapCat v4.18.38/bbd0d2aca3d470c20b33b1d4f95123335633bf26的extends/SendQzoneMsg.ts、DeleteQzoneMsg.ts、guild/GetGuildList.ts、GetGuildProfile.ts及core/apis/webapi.ts、data/qzone.ts已核对。两个频道_handle为空，标记compatibilityOnly、read和空对象参数Schema；无频道领域结果，不把null当业务完成。空间两个实际载荷Schema由Rust类型派生，默认上游受众与显式领域受众分别说明。

qzone post要求明确受众，支持1/4/16/64/128映射，selected/exclude须有最多100个JS精确QQ号，其余模式拒绝无效目标。支持最多9张本地图片，单张8MiB读取上限及实际JSON4MiB传输预算；qzone delete显式tid。受限聊天targets不能证明空间受众，调用前拒绝。共享API意图保证一次调用与缓存/未知不重发；发布非空tid才接受，缺tid为unknown。首个结果只投影tid，不持久内容/图片/tid/原生扩展，缓存tidAvailable:false。上游先上传再发布，失败可能留下图片，未宣称原子效果。

原创受管CLI九分支含文字/图片/部分好友、删除、原生拒绝、缺tid、切号、targets/send拒绝；输入与预览覆盖五类受众、精度/键要求和无配置状态。实际结果匹配严格派生Schema，缓存及离线query不联网；ledger无内容/tid/base64/原生秘密。social2、API3、CLI14、Skill3通过。全部为合成loopback及自有测试进程，没有操作真实QQ。

新增根命令触发Windows调试版Clap派生命令树构建栈溢出（离线api describe可复现）；定位在初始Cli::command。仅Windows命令定义首次初始化放入8MiB栈的短工作线程，OnceLock仍只缓存不可变定义，不转移应用、运行或凭据操作。修复后108叶/124帮助入口及全部输入/响应Schema回归通过，未留下诊断输出。其它平台保持原构建方式；macOS真实扩展仍待验证。

all-targets Clippy、fmt、架构/契约、Skill quick_validate、源码包白名单493文件、diff通过；Semifold仅规划18份变更集至0.2.0，未消费/发布。未重新跑完整suite或release构建，原完整证据仍按CHECK-152快照解释。空间可见性、实际发布删除/图片、频道缺口及真实macOS链路未验收。
