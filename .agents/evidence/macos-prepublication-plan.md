# 部署效果前的入口计划

文档编号：CHECK-114。日期：2026-10-24。关联：SPEC-008、TASK-006。起点：9d80fa3。

消除计划生成对已有适配器发布的依赖：AdapterPlan以相同生成器计算三文件组摘要和入口路径，并负责后续无覆盖发布/验证；plan_package_edit只计算受约束package结果，旧package_edit仍要求实际适配器验证。plan_source_package从Source的固定原始清单读取package及版本，不以调用者自报来源替代认证。未创建完整部署输入生产者或自动启动入口。

固定package读取在Inventory内通过原目录/文件描述符进行，限制1MiB，核对内容摘要、长度及原文件stamp，前后检查整树stamp；signing_file的可见性只扩大到同一Inventory模块的兄弟实现，无新增unsafe。Source仅暴露这个固定元数据读取，不开放任意路径读取。

合成验证：adapter父目录不存在时仍可得到稳定摘要与完整改写字节，整个规划不创建adapter或QQ.app；换Framework路径导致适配摘要和改写结果摘要变化。发布后实际入口与计划一致；内容篡改不因重新生成纯计划而被接受。部署合成用例使用未发布计划，未发布时原package保全并拒绝，发布后同一计划成功；既有失锁、原文件替换及其他资源变更反例继续覆盖。读取测试拒绝超限、原package改写、同字节新inode、无关树节点变化、祖先改为链接及过期预算。

本阶段仅原创合成材料，不挂载、复制、签署或启动真实QQ，不触碰会话。未将纯计划、摘要、适配器发布或package编辑等同完整部署/签名完成。

完整macOS门禁通过：22项Node、fmt、Clippy、343项Rust通过/2项忽略、arm64 release。源码包316项覆盖全部源码且排除内部/私人材料，受影响上下文链接/私人路径与git diff --check通过。Semifold变更集macos-prepublication-plan，status通过；Windows原生门禁未运行，仅本地提交、不推送发布。
