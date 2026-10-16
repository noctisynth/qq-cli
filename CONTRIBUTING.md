# 贡献指南

文档编号：CONTRIB-001。状态：Active。更新：2026-10-16。

## 上下文

阅读顺序：GOV-001 → DESIGN-001 → SPEC-001 → FACT-001 → RFC-0001 → TASK-001。
当前接入研究同时阅读 FACT-002、SPEC-003、DATA-001 和 CHECK-002，以固定版本审计为准。
原生加载器来源及维护调查见 FACT-003、DATA-002、CHECK-003；版本支持按证据类型区分，不跨模式推断。
目录按 specs、facts、evidence、rfcs、todos 分工；不把研究推断直接写成实现事实。

## 当前工具

初始化仅需 Git 与 Semifold。已验证工具版本及命令结果见 CHECK-001。
package.json 是 Semifold nodejs 解析器使用的元数据，无运行依赖、构建命令或可执行入口；
不需要 npm install，也不代表产品已经决定使用 Node.js。

常规检查：git diff --check、semifold config sync --check、semifold status、git status --short。
创建变更集示例：semifold commit -n describe-change -p qq-cli=patch --tag chore -m "说明变更"。
changeset 与源文件一起提交；git commit 与 semifold commit 是不同操作。

## 发布边界

当前只初始化本地仓库，没有远端和 CI/CD。Semifold 配置见 SPEC-002。
不要为验证而执行 version 或 publish；它们可能消费变更集或改变版本。
产品运行时、原生工具链、测试矩阵和发布渠道在确认实现路径后再建立。
