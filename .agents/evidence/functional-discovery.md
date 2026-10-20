# 联系人与会话发现功能验收

文档编号：CHECK-039。状态：当前真实环境正常发现/详情通过。更新：2026-10-20。
关联：CHECK-038、SPEC-004、TASK-002。CLI代码基线3bb3b5c，固定Framework v4.18.38；原签名实验实例status为ready/online。用户要求其余功能继续在真实环境验收，仅功能性范围，不开展稳定性专项。

contact/group/conversation list均先limit 2，再用各自签名cursor续页limit 10。分别取得2+5联系人、2+7群、2+3最近会话，均complete:true、页间无重复，覆盖分别为friends/groups/recent。输出原文、真实ID/名称只留忽略目录，不提交目录快照或联系人夹具。

实验联系人contact get成功，与按该ID过滤的contact list唯一结果一致。指定实验群group list按ID过滤取得唯一结果，群详情/成员正常证据沿用CHECK-038。conversation get已知群会话成功；按其实际名称过滤能找回目标，不借最近会话查询扩为好友/群搜索。

当前read::filter明确排除conversation字段，所以conversation list --query数字ID返回空，而同一目标conversation get成功；这不代表会话不存在。已按名称过滤验收且文档注明范围；contact/group的独立userId/groupId字段可匹配ID。没有修改过滤代码或引入新查询语义。重复同名候选不自动选取发送目标，本轮没有此类真实同名样本或多实例歧义试验。

用户将转发缺项暂按“可能无法取回包含当前账号发言的条目”跟踪。仅记录假说：CHECK-036旧样本4条窗口中缺失的3条均属当前账号，而后端所返1条及CHECK-037新样本所返5条均非当前账号。该关联未证明所有含自身转发均失败或其余转发全部正常，不把未知根因写成事实或增加运行时阻断。

普通发现7条命令的已测正常路径可用，未发现需要修复的业务代码；特殊后端类型/同名/并发及稳定性未由此验收。用户文档patch:chore对应范围说明，内部上下文不单独建changeset。后续等待本轮写入内容授权，继续私聊回复/群提及和附件；未授权前不执行这些新写入。
