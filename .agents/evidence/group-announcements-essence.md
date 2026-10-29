# 公告与精华快照用户链路

文档编号：CHECK-152。状态：R08公告与精华切片离线通过。日期：2026-10-29。
关联：RFC-0009、SPEC-009、TASK-007、CHECK-151。

固定NapCat v4.18.38/bbd0d2aca3d470c20b33b1d4f95123335633bf26的go-cqhttp/SendGroupNotice.ts、group/GetGroupNotice.ts、DelGroupNotice.ts、GetGroupEssence.ts及core/group/WebAPI来源已核对。group notices有界投影正文、图片ID/尺寸和存在的新成员/置顶/阅读字段，任意settings不透传。group notice支持明确文字/本地图片及原生布尔/类型选项，8MiB文件快照为base64，不转发本机路径或跟随URL；发布前的图片上传可能留下资源，失败不自动清理/换键重发。删除源是void调用，回执只证明接受，不证明删除效果。

group essence-list归一化有界内容并保留序列/random/原生缓存ID、发送/操作人。上游为缺失原消息制造的ID不当m1，快照归一化显式无签名来源，图片/文件/语音/回复/转发不签发m1/f1、不解析资源或跟随URL。group essence-remove用明确group/seq/random原生元组移出，检查parseInt可精确表达的有符号整数；与m1精华动作共用delete_essence_msg键域，异参冲突。

原创受管CLI十一分支覆盖公告首批/精华及超精度元组拒绝、文字/图片发布、删除/元组移出、发布拒绝、只有send权限的发布、目标/read拒绝。预览无网络/意图，8MiB上限与输入依赖离线检查；写请求重复不联网，实际输出匹配派生Schema，状态无正文/图片数据/原生扩展。精华六类内容显示不产生可用引用，置顶false不丢失，未报告列表完整。

首次CLI回归发现Windows调试主线程栈溢出：静态诊断标记定位到深应用栈中的语义校验重新构造整个Clap树。仅拆公告Args尚不足；主入口、Schema与校验现共用由原定义构建的不可变树，各取独立克隆，缓存不含ArgMatches/用户参数。公告多字段Args单独封装，公开形状/必需/默认/冲突仍由实际Clap产生。临时诊断已移除，未扩大线程栈或引入unsafe。

修复后CLI14、公告2、Skill3回归通过，随后完整cargo test --offline --locked -j1串行通过，覆盖原消息/媒体/事件/持久请求及合成启动停止/恢复链路。all-targets Clippy、结构/契约、Skill quick_validate、源码包白名单481文件、fmt/diff通过。Windows x86_64 release构建通过，产物100叶命令、公告离线预览与精华Schema冒烟通过。Semifold仅规划15份变更集，未消费或发布。

本轮未操作真实QQ，也未执行macOS新链路验收；真实图片格式/公告效果、精华/待办/签到实际效果仍须按TASK-007授权验收。其余能力、完整混合格式、事件及流式继续开发，不把离线回执或构建当完整交付。
