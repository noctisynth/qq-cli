# 自动检查、Semifold持续交付与上下文边界

文档编号：RFC-0007。状态：Accepted。日期：2026-10-22。
关联：SPEC-002、SPEC-005、TASK-005、PLAN-001。

用户确认Semifold承担CD；采用quality、semifold-status、semifold-ci三个workflow的职责划分。Windows为唯一已支持平台，质量检查复用原创PowerShell脚本，PR不运行QQ，不下载或打包NapCat。固定Rust工具链从rust-toolchain.toml读取，Cargo先fetch --locked再离线验收。

Semifold在main上串行执行ci，负责release分支/版本PR、合并后的tag/GitHub Release及资产上传；workflow不自行推导tag或重复实现version/publish。保留Cargo和Semifold registry publish=false，开启github-release并配置自产Windows x64 ZIP及SHA256SUMS。私有包会跳过resolver publish命令，因此workflow在semifold ci前准备资产，不能把构建藏在不会执行的prepublish钩子里。post-version仅cargo update --workspace --offline同步自身锁文件版本。

文档边界：内部架构DESIGN-002和完整计划PLAN-001从docs迁入.agents；编号保持，现有公开使用指引保留README/CONTRIBUTING和嵌入Skill。Cargo源码包与二进制ZIP均排除.agents、私人运行/研究目录及上游材料。不在公开上下文写参考工作区名称或本机路径。

Semifold setup沿既有0.3.x兼容默认策略获取latest，两处workflow一致；其余通用Actions采用明确主版本。变更只创建本地workflow和发布契约，不配置远端、不推送、不运行本地semifold ci/version/publish，不触发真实QQ操作。GitHub托管runner/Release实跑待远端就绪，不将本地验收冒充在线成功。
