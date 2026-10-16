# Framework 文档与安全结论复核

文档编号：CHECK-004。状态：Static research complete。更新：2026-10-16。
结论见 FACT-004；固定 NapCat 发布基线仍为 DATA-001。

- 读取官方文档仓库非浅历史，固定观察提交，核对 Framework.md 20 次变更及完整 diff。
- 核对维护警告、旧 Installer 删除和现行简化提交，未把旧 Once/Installer 与原生入口混淆。
- 读取官方 issue #1375/#1398 的 Member 回复，确认历史推荐 napiLoader 实现有头加载。
- 通过搜索 API 定位问题；记录限流及全站历史搜索中止的范围限制，不宣称穷尽所有讨论。
- 读取 LiteLoader 官方归档标记和风险警告、NapCat 安全页及历史 Discussion #517。
- 读取固定发布源码的两种模式调用点、Napi2Native 导出类型及配置；复核 Framework ZIP 调用点。
- 核对 Bypass 引入和配置提交的主题/说明，区分反检测意图与原生实际效果。

没有运行代码、验证 Bypass 效力、测量封号概率或取得平台授权。
无 fork、QQ 操作、聊天/账号读取、发送或维护者联系。研究检出和原始 diff 留在忽略材料。
收尾检查包含文档编号、隐私、Git 空白与 Semifold 配置；不把文档检查写成 QQ 接入测试。
