# 贡献指南

文档编号：CONTRIB-001。状态：Active。更新：2026-10-17。

## 上下文

阅读顺序：GOV-001 → DESIGN-001 → SPEC-001 → FACT-001 → RFC-0001 → TASK-001。
当前接入研究同时阅读 FACT-002、SPEC-003、DATA-001 和 CHECK-002，以固定版本审计为准。
原生加载器来源及维护调查见 FACT-003、DATA-002、CHECK-003；版本支持按证据类型区分，不跨模式推断。
目录按 specs、facts、evidence、rfcs、todos 分工；不把研究推断直接写成实现事实。
当前 CLI 实施阅读 [首版技术方案](docs/cli-design.md)，内部范围、选型和任务分别见 SPEC-004、RFC-0002、TASK-002。

## 当前工具

需要 Git、Semifold、rustup 及 Windows MSVC 链接工具；Rust 工具链由 rust-toolchain.toml 固定为 1.98.1。
Cargo.toml 是唯一包版本来源，Semifold 使用 rust 解析器，旧 package.json 已移除。
Cargo.lock 锁定依赖；reqwest 仅开启 blocking/json，M0 不创建 HTTP 客户端或发请求。
现有实验探针不是 CLI 产品代码，不作为正式安装或使用入口。

首次准备可联网执行 cargo fetch --locked；随后运行以下离线检查：

```powershell
./scripts/check.ps1
```

脚本依次执行 fmt、clippy（警告视为错误）、8 个 CLI 子进程测试、release 构建和源码包白名单检查；构建/测试/包清单使用 --offline --locked。
测试只用合成参数与本地监听器检查无连接，不需要运行 QQ/NapCat，也不读取真实配置或聊天。
业务命令尚未实现，返回退出码 6 是 M0 的预期结果；参数错误返回退出码 2，help/version 返回 0。

常规检查：git diff --check、semifold config sync --check、semifold status、git status --short。
创建变更集示例：semifold commit -n describe-change -p qq-cli=patch --tag chore -m "说明变更"。
影响发布内容、产品行为或使用方式的代码/配置变更需要 changeset，并与源文件一起提交。
用户文档（README、使用/开发指南、公开技术方案）使用 patch:chore；纯 Agent 上下文不创建 changeset。
混合提交记录用户可见的变化；历史已清除条目不回填。git commit 与 semifold commit 是不同操作。

## 发布边界

当前为本地仓库，已完成 CLI M0 与原版 Framework 的独立有界实验，尚无后端功能、远端或 CI/CD。Semifold 配置见 SPEC-002。
不要为验证而执行 version 或 publish；它们可能消费变更集或改变版本。
原生交付完善及真实 CLI 回归属于后续阶段，发布渠道单独确定；不默认执行 cargo publish。
