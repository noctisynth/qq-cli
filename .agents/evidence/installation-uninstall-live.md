# 安装与卸载正常功能验收

文档编号：CHECK-046。状态：固定包安装/缓存/引用保护/卸载/重新安装通过。更新：2026-10-20。
关联：PLAN-001、SPEC-004、RFC-0003、TASK-002、CHECK-025、CHECK-029、CHECK-043、CHECK-045。基线746eb8c；用户本轮只要求完成安装/卸载验收，并倾向显式安装职责。不启动/登录/停止任何QQ，不扩展聊天授权。

职责：install显式取得固定安装，installations查询，uninstall移除无引用的自有安装。start/update不隐式联网或新增安装；Agent按缺安装指引执行install后继续原任务，无需逐步询问用户。update已有结构化nextAction且目标缺失不停止原run。运行配置/代次准备与安装获取是不同职责；旧文档候选中的自动目标准备已由此取代，不将取消项勾作实现。无需改NapCat或业务代码。

真实基线：37/38两份固定Framework安装，37无引用，38仍有两份历史失败/未知登记引用。使用不存在的--from路径执行37 install，cached:true，说明受检完整安装复用未打开来源。38 uninstall返回INSTALLATION_IN_USE、count2；不删除或修改那些登记，也不把未知实例当可自动清理。

无引用37 uninstall前验证解析后的绝对目录直接位于自有installations根内、目录名等于已登记目标ID；随后对全部状态文件作摘要快照。公开CLI返回removed:true，681个目标安装文件移除（680个包成员及1个标记），其它1167个原文件摘要不变，除registry外无原文件改写，原请求/运行/凭据引用/其它安装保留。

重复uninstall返回INSTALLATION_NOT_FOUND，契约是原卸载日志完成后不将任意缺失ID报告成功，不混同remove的持久注销幂等凭证。随后使用CHECK-025原审计完整ZIP显式install --version VERSION --from ARCHIVE，cached:false，原完整来源/长度/摘要/解包检查生效；两份安装按ID排序后的登记与验收前相同，全部实例登记完全不变，1167个保护文件仍逐项同摘要。不执行新的QQ启动或会话操作。

私人命令/安装位置/真实登记及摘要清单只留忽略目录；无上游代码或二进制纳入Git。正常实际下载基线见CHECK-029，37在线来源不可用及本地固定包导入见CHECK-043，本轮不把离线导入冒充新联网下载成功。不扩大为全部网络/故障/清理或稳定性验收。

离线安装边界8项及Skill发现/导出2项通过；正常release重建通过，离线skill get确认已内嵌显式install指引且在9000字节预算内；架构/217文件源码包、diff/隐私/编号和Semifold检查通过。仅用户方案/指南变更使用patch:chore，纯Agent上下文无独立changeset。原67/100混合计数保留，功能子项表补齐当前验收，不宣称整体验收完成。
