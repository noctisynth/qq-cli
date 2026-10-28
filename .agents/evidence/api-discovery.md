# 离线接口目录与详情

文档编号：CHECK-138。状态：离线切片通过。日期：2026-10-29。
关联：RFC-0009、SPEC-009、TASK-007。

原创目录取固定NapCat v4.18.38注册名，来源见SPEC-009；178项唯一名称，只有原16项已接入效应分类，其余unknown，所有参数Schema尚未核对。
新增api list/describe，不读取配置、凭据、实例或QQ；固定版本/查询绑定游标，非法路径/action拒绝，未知action返回known:false而非封死未来调用。
actual Serialize/JsonSchema类型提供目录/详情输出；Clap提供ActionName语法及输入Schema。发现Skill和README仅说明已实现命令，未开放api call/stream。

验证：cargo check离线通过；API2、CLI14、input_contracts1、public_contracts5、skills3共25项串行通过；all-targets Clippy、结构/单源契约门禁、Skill quick_validate、fmt与diff检查通过。
API合成测试覆盖分页无重复/缺漏、跨查询游标拒绝、未知action/兼容后缀效应、action路径注入拒绝与实际输出符合派生Schema。Skill完整导出/原目标保全和命令路由测试通过。
rebase后固定依赖缓存不足，执行cargo fetch --locked补齐，未改变Cargo.lock。RTK封装未及时返回诊断的两次测试已中止，最终使用rtk bypass取得完整编译和上述通过结果，不计中止为通过。

限制：仅离线发现，不计新增业务action已接入，不证明上游实际可用、参数Schema或全部效应已核对；未执行真实QQ操作、推送或发布。api call及持久请求为下一独立切片。
