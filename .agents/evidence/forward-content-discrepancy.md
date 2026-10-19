# 合并转发窗口与API内容差异

文档编号：CHECK-036。状态：真实完整性验收失败，后端返回缺项。更新：2026-10-19。
关联：CHECK-032/035、SPEC-004、TASK-002。CLI代码f2eb699，Framework v4.18.38，napcat.mjs SHA256 f79260cb7e6c4b579a4346ffd39a91a33ea73f9b7cd44d9ce204ab471096601a。

用户在原实验私聊手动发送合并转发，并提供打开后的截图：4条测试文本，三个为当前登录账号、一个为对方。截图及真实身份/正文不进入公共文档或测试夹具。

从最近3条有界历史定位该forward，以同一签名消息引用分别调用get及get --expand：普通forward expanded:false，展开为expanded:true、仅1个文字节点。受原签名进程/Job/身份/会话保护的只读诊断进一步比较get_msg原始内嵌content及get_forward_msg资源，均仅1条且内容相同；与CLI展开文字一致，该条对应截图第三条。用户打开QQ窗口后同一消息再次expand仍1条。不是CLI归一化丢弃已返回条目，也不是未知类型被静默滤掉。

固定源码观察：multiForwardMsgElement先调用getMultiMessages/getMultiMsg，仅列表为空才FetchForwardMsg协议兜底；get_msg默认解析转发；numeric get_forward_msg同样回到本地消息解析。不完整非空列表会绕过空列表兜底，但这只是可能机制，未观测原生调用完整结果，不能定为底层根因。现有受审API未发现可直接取得resId并强制协议兜底的只读入口；不修改NapCat、不调用发送接口或自研注入绕过，不用原会话历史猜测缺失转发节点。

上游相关报告（2026-10-19检索，可变来源）：https://github.com/NapNeko/NapCatQQ/issues/1174 ，报告v4.8.95自身发言缺失；维护者2026-01-23建议get_msg、表示可能已修复并关闭not planned。与本次缺失发送者模式高度相似，但不是当前版本根因证明。当前v4.18.38两接口实际仍不能恢复本样本4条。

恢复被用户意外暂停的工作后复核未有额外发信/实例操作；release重新构建通过，重建二进制skill get read确认包含后端返回节点的完整性限制，QQ会话未重启。

功能状态：普通get/下载/search/export此前通过不变；完整多条转发读取未通过，先前“待样本”变为“已取得样本但后端缺项”。expanded表示已展开后端返回节点，不代表QQ显示全部内容。接口无可靠总数/缺项标志，不能把所有单节点转发判成错误。更新Spec、用户文档与Skill语义，不新增运行时硬阻断，不以声称修复或假完整输出掩盖失败。Skill发现/导出2项测试、diff/隐私及Semifold检查通过；未进行稳定性专项，原67/100混合计数不变。
