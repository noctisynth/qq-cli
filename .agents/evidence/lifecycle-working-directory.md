# 实例预检与原生工作目录修复

文档编号：CHECK-041。状态：列表/在线复用通过，restart预检拒绝；CWD修复离线通过，真实后继未验收。更新：2026-10-20。
关联：CHECK-040、SPEC-004、TASK-002。用户确认此前本轮对照无误并授权转入实例功能验证；转发上游缺项本会话暂缓。仅操作原实验实例，其它QQ/未知实例不停止接管。

instances/installations正常返回3个登记实例、1个安装；显式start复用原在线实例ready，不新开QQ。restart在停止前INSTALLATION_MODIFIED拒绝，随后status仍ready/online，未执行stop/remove/update/rollback。

原签名i1清单只读核对：固定成员全在且摘要一致，额外1个Tencent/QQ下的运行日志。真实子路径/账号/正文只留忽略目录。原Bootstrap以安装目录作为CWD，存在让运行文件落入安装区的隔离缺陷；不能由此声明已观测原生日志写入的完整根因。

按SPEC-004修复：bootstrap显式接收同run已验证运行目录作为CWD，拒绝与QQ/安装目录上下级重合。三参数不改，dispatch沿verify_prepared绑定，CWD仍进入输入摘要和最终同锁复核。不改变已运行QQ的CWD，不自动移动删除旧日志、不放宽严格安装清单。

launch 3项及runtime_store 20项定向、fmt/all-targets clippy/release通过；新CWD未实际启动QQ，不声称已经解决实际日志生成。未重复完整套件或稳定性专项。doctor独立观察兼容unknown/NO_EXACT_FRAMEWORK_EVIDENCE，reviewed_witnesses仍为空；首次显式组合试验不自动成为支持记录。

下一步：保全旧运行日志并恢复安装完整性、补齐可复核的本地精确启动证据，再做stop/start/restart与版本维护。不得先stop后赌恢复、伪造支持或降低所有权/完整性门禁。原会话保留在线；真实生命周期大项未勾选，67/100混合计数不变。
