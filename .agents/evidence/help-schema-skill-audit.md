# 帮助、Schema与Skill对照

文档编号：CHECK-049。状态：当前公开面核对完成；严格字段及单一契约来源仍有差距。更新：2026-10-20。
关联：PLAN-001、SPEC-004、TASK-002、CHECK-047、CHECK-048。基线4fbbf7a；按用户顺序在双实例及事件正常功能验证后进行离线核对，不扩展真实发信。

来源事实：cli/args的Clap树同时驱动解析和帮助；cli/registry提取输入字段/默认/必填并手工补跨参数约束。cli/schema手写输出JSON Schema，业务不少结构以serde_json对象构造；不是由同一输出类型自动生成。Skill为手写Markdown、编译时内嵌，任务路由与命令注册表自动核对。测试发现漂移而不能证明所有路径天然同源。

离线核对34叶命令、45个根/中间/叶帮助入口，均可在缺失配置参数下打印帮助而不访问后端。所有inputSchema/responseSchema及可用preStreamErrorSchema可编译，Schema公布的cliFlag都在对应帮助中。resume依赖conversation的Clap拒绝与JSON Schema约束吻合。6个Skill任务正向路由合法，反向覆盖所有业务命令（schema/skill自省命令单列），导出8文件及不覆盖、单任务正文预算继续通过。

修正：shared说明区分JSON meta.scope和流ready.scope；仅消息/媒体引用及查询游标推导实例，event checkpoint须原选实例及显式会话；流退出7看summary/gap而非pageInfo。公开方案update表补--verify-compatibility/--timeout及rollback总截止。shared增长后runtime合并文本恰到9000字节触发旧预算测试，精简重复说明，未放宽预算或移除关键授权/未知结果规则。

剩余精度：doctor、schema.list、schema.get、skill.list、skill.get、skill.export、napcat.install、napcat.installations、napcat.uninstall、napcat.instances、message.export共11个responseSchema仍为通用data envelope，没有完整约束data字段。编译/帮助测试通过不代表其严格字段契约已完成；作为原交付Schema门槛的细项继续未勾选。输出类型化并同源生成Schema是候选整改方向，尚未由本核对宣称实现或冻结技术方案。

CLI13项（新增所有Schema编译/所有flag帮助对照）及Skill3项（新增反向覆盖及流规则），共16项本阶段通过；与CHECK-048重叠项不重复累计，本轮唯一用例31项。fmt/clippy/release、架构/219文件源码包及最终内嵌指南核对通过。正常双账号仍在线，未重发消息、不停止实例。本核对不提升完整交付/全部登录挑战/生产notice或稳定性门槛。原混合业务计数68/100保持。

用户文档/Skill与测试使用patch:chore changeset；纯Agent记录不额外创建changeset。私人回归材料和运行身份不进入公共上下文。
