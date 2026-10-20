# 事件契约整改

文档编号：CHECK-056。状态：切片通过。日期：2026-10-21。
关联：TASK-004、RFC-0006；基线9b49ee8。

ready/message/notice/gap/summary由实际Event枚举序列化并派生Schema；六类Notice及其subType、Recovery统计复用历史停止原因类型。受支持通知必需具体会话，未知通知只保留可空会话与supported:false；不穿透原始后端载荷。Sink仅接收实际Event，预流错误另接收Failure；底层有界记录编码器可序列化明确调用方类型。删除event_stream手写Schema，schema模块仅剩实际类型生成入口。

30项事件/事件Schema/监听/CLI/有界输出合成测试通过；最后收紧已支持通知会话要求后events及event_schema共7项再通过。全目标Clippy以及最终库Clippy、fmt/架构门禁通过。消息稳定ID、检查点、gap和summary预算行为保留。原生输出测试使用完整实际Gap/Summary合成记录验证取消与阻塞，不再构造残缺消息外壳。没有真实QQ操作。

输入约束单源化、最终结构防回退门禁和打包整体验收仍未完成，不能以本阶段宣称产品整体交付。
