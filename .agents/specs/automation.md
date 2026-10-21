# 仓库自动化与发布契约

文档编号：SPEC-005。状态：Active。日期：2026-10-22。
关联：RFC-0007、SPEC-002、TASK-005。

- PR与手动触发运行可复用Windows质量检查；main推送先通过相同质量门禁再运行Semifold。检查包括结构/单源契约、fmt、Clippy、完整串行合成测试、release构建和源码包范围，不读取真实QQ会话。
- PR状态由semifold status报告；同仓PR允许写评论，fork PR只做无写权限质量检查，不使用pull_request_target执行贡献者代码。
- Semifold独占版本、发布分支/PR、tag、changelog与GitHub Release编排。ci输出保留semifold-version/semifold-publish结构化事实，不从tag或日志重新推断发布结果；发布阶段不并发取消。
- 交付目标Windows x86_64-pc-windows-msvc，版本只读Cargo.toml，产物ZIP仅原创CLI、README及Skill；独立SHA256SUMS校验ZIP。发布文件不含NapCat/QQ、研究材料、.agents或凭据。
- Cargo及Semifold禁用注册表发布，但允许Semifold创建GitHub Release及上传显式资产。post-version更新workspace锁记录，保持第三方锁定版本。发布所需GitHub权限仅授予受信main发布job。
- 内部上下文统一.agents，源码包不再包括docs；公开文档只说明使用、开发命令和仓库自动化。尚未执行远端CI、版本消费、推送或发布，必须如实保留该验收边界。
