# 单源契约基础与安装管理验收

文档编号：CHECK-050。状态：切片通过。日期：2026-10-21。
关联：RFC-0006、TASK-004、SPEC-004；基线提交83798b9。

schemars锁定1.2.3（MIT），schemars_derive同版MIT；新增传递依赖serde_derive_internals 0.30.0及dyn-clone 1.0.20均MIT OR Apache-2.0。Cargo.lock固定解析；未升级已有包。生成器明确draft2020_12及for_serialize，不采用默认反序列化契约。说明参考公开API文档，发布锁定以Cargo.lock为准：
https://docs.rs/schemars/1.2.3/schemars/generate/struct.SchemaSettings.html

实际业务构造Installed、Uninstalled、Installation、RecordedInstance，并通过封闭PublicData分派到共享Reply容器；字段定义同时派生Serialize与JsonSchema，未复制Schema专用DTO。Schema注册仅映射命令和类型。版本/布尔标记由同一个关联常量供序列化和Schema使用，LocalId由运行时校验构造。napcat.install/installations/uninstall/instances由通用兜底响应变为派生契约；剩余7个通用响应及其他手写响应仍待整改。

离线验证：
- cargo check --offline --locked -j1 --all-targets通过。
- 定向public_contracts/installations/cli共24项通过；最终meta字段省略注解追加后，public_contracts三项再次通过。
- 完整串行离线回归312项通过（45个测试结果，包括零测试目标），无失败。
- cargo clippy --offline --locked -j1 --all-targets -- -D warnings通过；release构建及fmt检查通过。
- 架构门禁通过；源码包白名单224文件通过。包元数据缺少许可等既有提示仍待发布审计。
- 合成真实安装入口与CLI列表输出对照派生Schema；负例覆盖缺字段、多字段、错误常量、非法标识与不可能输出的显式meta:null。未操作真实QQ或运行实例。

初次offline缺少新依赖，完成固定依赖获取后离线验证；初次derive约束属性语法经编译发现并改为支持的regex属性。旧Windows PowerShell执行策略拒绝检查脚本，使用现有pwsh运行成功，未修改系统策略。

限制：LegacyWire、旧success(Value)、分页Value、错误details/nextAction及未迁移命令的手写Schema明确保留为临时桥；共享错误容器已派生，但错误细节未完全收敛。输入约束与Clap整合仍待后续切片。通过此切片不代表TASK-004或产品整体完成。
