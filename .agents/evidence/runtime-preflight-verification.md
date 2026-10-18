# 运行前预检与分层诊断验证

文档编号：CHECK-014。状态：元数据/证据判定内核定向验收，真实QQ组合未验收。更新：2026-10-18。
关联：PLAN-001、SPEC-004、RFC-0003、TASK-002、FACT-001、FACT-002、FACT-003、CHECK-013。

## 已完成

- 增加平台、QQ安装元数据、兼容证据判定和doctor模块；原工程仍禁止unsafe项目代码，Windows调用封装在锁定依赖中。
- QQ注册表定位仅读取安装项，不执行UninstallString；解析绝对卸载程序路径，路径和版本元数据拒绝穿越/链接/远程路径。
- 检查两种固定版本布局，读取实际curVersion、package版本/build及main；不使用上游默认模板冒充实际QQ build。
- 对QQ入口、所选major.node、入口脚本、配置/包元数据计算摘要；PE头偏移、文件/元数据大小和截止有界，元数据前后变化拒绝。
- 所有受检文件与QQ安装选择、系统build/原生及进程架构、模式、NapCat提交/完整包/加载器摘要进入精确兼容键。
- 仅精确FrameworkStartup证据可形成supported/unsupported观察；静态、Shell、缺失与冲突证据不提升为Framework已验收。当前内置完整真实元组为空。
- doctor分别报告产物、适用依据、安装元数据、兼容证据和所选实例API；API成功不提升兼容结论，不创建状态、密钥或启动QQ。

## 验证范围

QQ元数据6项：两种布局、摘要失效、版本冲突/缺失/穿越、PE异常/架构/截止、卸载路径解析、Windows原生系统版本与架构读取。
诊断4项：精确证据与冲突、API/兼容分离、启动前置条件、实例/QQ候选歧义。
新增测试只读原始合成安装目录、假探针和本机系统版本/架构；未扫描真实QQ安装、读取其会话、启动/注入或发信。
此阶段完整离线流水线126项，通过fmt/clippy/test/release和源码包检查（88文件）；后续总量及其他模块证据以CHECK-013为准。

## 固定依据与边界

读取FACT-002固定包及对应源码的QQBasicInfoWrapper、getQQVersionConfigPath、getQQPackageInfoPath、getMajorPath与原版BAT。
BAT通过卸载项定位QQ.exe；元数据辅助函数使用版本配置和所选版本package/major文件，且上游存在版本模板及环境覆盖。
本项目独立实现受检路径/格式适配与摘要逻辑，不复制上游实现，也不接受这些环境覆盖作为可信版本来源。
实际启动还须约束子进程环境，避免NODE_OPTIONS、版本路径或工作目录覆盖使受检对象与实际加载对象不同；尚未完成启动绑定。

依赖锁定windows-registry0.6.1、windows-version0.1.7、winsafe0.0.29（kernel）；本机编译和原生系统信息测试通过，不替代完整依赖许可/安全审计。
API参考：[GetNativeSystemInfo所在固定源码](https://github.com/rodrigocfd/winsafe/blob/71ed88c2a0d18b03ee452f6d4261f4c22f443483/src/kernel/funcs.rs)、
[windows-registry](https://docs.rs/windows-registry/0.6.1/windows_registry/)、[windows-version](https://docs.rs/windows-version/0.1.7/windows_version/)。
检索2026-10-18；网页为版本化参考，实际源码由Cargo.lock摘要约束。
winsafe精确函数文档页未能由检索工具读取；版本包VCS标注上述提交，本机源码、编译和原生系统信息测试提供本次直接依据。
固定源码网页也未抓取成功，链接按版本包VCS定位，不当作远端逐字节核对证据。

## 未完成

没有将旧Shell报告、旧API实验或文件存在转换为完整兼容证据，也没有自动探测未知组合是否能注入。
注册表发现的安装元数据不代表已运行进程映像；实例与QQ安装的持久绑定、PID/更新失效、全部预算和原生安装布局需继续验收。
doctor的canStart只表示受检前置条件，不等同已启动、全部业务能力、用户操作授权或账号安全。
当前适用授权未确认、真实完整元组为空；不以这些测试作为首次真实加载或完整产品交付依据。
