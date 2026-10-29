# 收藏摘要与自定义表情元数据

文档编号：CHECK-153。状态：R09首段六action离线通过。日期：2026-10-30。
关联：RFC-0009、SPEC-009、TASK-007、CHECK-152。

固定NapCat v4.18.38/bbd0d2aca3d470c20b33b1d4f95123335633bf26的extends/CreateCollection.ts、GetCollectionList.ts、FetchCustomFace.ts、CustomFace.ts，以及core/apis/collection.ts、apis/msg.ts、Collection/Msg服务已核对。favorite create以当前账号建立条目，UTF-8原始数据只取明确literal/file/stdin；实际非媒体JSON总预算1MiB，原生SDK形状仍unknown，回执只证明OneBot接受，不伪造收藏ID或渲染效果。

favorite list有界投影首批摘要，双状态result/errCode任一非零拒绝；时间保留字符串，hasMore原样，complete/bodyAvailable均false。只有已核对richMedia标题/brief进入领域结果，其它summary/author/正文不透传。原消息来源无法证明，受限targets时拒绝，不信任调用方额外group_id。

emoji list使用详情接口，URL查询为同业务的直接替代，完整性false保留源丢弃状态/空结果的限制。资源ID、可精确表达的emojiId、MD5/描述/URL和原生标志按类型投影，isExist仅称nativeExists，不冒充文件下载可用性；原生本地路径/OCR词不暴露或使用。remove只按resId/ids删除，md5虽在源Schema却被忽略；describe明确三字段定位，0可表达、空描述清空，超JS精度输入拒绝。六项参数Schema取实际Rust载荷类型，领域不加入动态JSON。

原创受管CLI十四分支覆盖摘要/原生两状态错误与范围拒绝、表情/超精度ID省略、创建三种输入/拒绝、删除/描述和read/send拒绝；成功或拒绝重放无网络，状态扫描无正文、路径或原生扩展。离线检查输入互斥、UTF-8、实际JSON转义后超预算、请求键及无状态预览，实际输出匹配Schema。API十一项单测含自有仓库/伪会话声明策略；CLI14、输入/截止、API/公告及Skill3回归通过。

Skill新增assets任务，共10任务/13导出文件，路由覆盖105叶命令及120帮助入口；首次导出回归因编辑改变入口换行为CRLF失败，恢复既有LF后通过。all-targets Clippy、结构/契约、Skill quick_validate、源码包白名单487文件、fmt/diff通过；Semifold仅规划16份变更集，未消费/发布。上次完整离线suite与Windows构建仍按CHECK-152快照解释，不作为本切片重新完整运行。

add_custom_face的文件路径/快照与SDK源文件生命周期仍待独立实现，未把raw可访问当领域上传完成。未执行真实QQ或macOS新增链路验收，不证明条目渲染、删除/描述实际效果或返回列表完整。
