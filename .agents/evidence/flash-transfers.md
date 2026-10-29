# 闪传与有界目录影本

文档编号：CHECK-159。状态：R12八action协议/领域链路离线通过。日期：2026-10-30。
关联：RFC-0009、SPEC-009、TASK-007、CHECK-157/158。

来源固定NapCat v4.18.38/bbd0d2aca3d470c20b33b1d4f95123335633bf26：file/flash八处理器、core/apis/flash.ts、data/flash.ts与NodeIKernelFlashTransferService.ts。逐载荷派生Schema；实际创建结果为createFlashTransferResult、分享链接为对象、文件URL为transferUrl，与示例差异分别处理。八领域命令覆盖创建、状态/首批文件、解析/链接/URL、发送、默认目录下载任务；不将任务接受当完成，不伪造消息引用。

目录输入复用受管影本/配额/签名意图，128MiB/16层/256子项，拒绝链接/特殊文件/状态重叠。排序相对路径/类型/内容清单哈希，复制后复核；目录类型进入私有HMAC指纹且认证持久保留。原文件指纹与旧认证字节不变。配额标记只计量，不授予所有权；普通prune保留异步输入。Windows扩展路径复制失败已在测试中修正，仍用绝对原输入路径进入既有受控复制，规范化路径只作状态重叠比较。

原创合成tests/flash.rs两测试含22种受管模式：八任务、私聊/群与raw优先级、创建file/dir和内容绑定、持久来源/同键缓存/变内容冲突、native失败/缺确认、错误文件集身份、read/send/media/targets拒绝、结果Schema与秘密不落ledger。目录/配额五单测覆盖稳定内容、空子目录、篡改恢复、持久prune、不合规树和源/状态重叠；全部通过。流式测试新增get_record通过api stream仍受mediaDownload约束，普通JSON与流式共用效应帮助函数，无跨入口绕过。

CLI14、Skill3、文件2（27场景）、附件缓存3及流式2（新增媒体权限模式）回归通过；check/clippy全targets、fmt、architecture/contracts/package（520文件）、Skill quick_validate和diff检查通过。新增changeset flash-transfers，semifold status共22项，规划0.2.0，未消费/发布。未执行真实QQ、未验证原生异步生命周期/下载目录/缩略图与macOS；未以离线成功替代真实验收。累计全套及release将在R13后另验。
