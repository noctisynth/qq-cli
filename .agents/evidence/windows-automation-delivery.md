# Windows自动检查与Semifold持续交付验收

文档编号：CHECK-065。状态：本地实现与验收通过，远端未运行。日期：2026-10-22。
关联：RFC-0007、SPEC-002、SPEC-005、TASK-005、CHECK-064。

实现：quality.yaml在PR/手动/可复用调用执行Windows-2025质量及打包；semifold-status.yaml仅对同仓PR写发布计划，fork不运行有写权限状态job。semifold-ci.yaml在main推送后先调用同一质量workflow，再于Windows受信job构建资产并执行semifold ci。CD串行且不中途取消，保留semifold-version/semifold-publish原始结构化job输出。无pull_request_target，不接真实QQ；发布job只申请contents/PR写权限，不需registry/OIDC权限。

Semifold0.3.4及配套已安装engine源码核对：私有包跳过注册表命令，但可创建GitHub Release并上传显式资产，因此Cargo与Semifold publish=false保留，github-release=true；资产在workflow调用ci前生成，而非无效的私有包prepublish。post-version配置cargo update --workspace --offline。Rust版本只读rust-toolchain.toml，workflow先fetch --locked，再复用离线脚本，不复制第二份版本常量。

scripts/release.psm1封装原创CLI/README/Skill的显式文件集、PE x64校验、无覆盖ZIP写入、SHA256SUMS和路径白名单；package-release.ps1组合Cargo元数据、明确x86_64-pc-windows-msvc目标构建及exe版本匹配。check-release.ps1在独立临时目录测试正常包、覆盖拒绝、哈希篡改、错误架构和路径穿越拒绝；使用实际manifest/lock副本执行post-version命令，仅本包版本改为合成值，其余锁文件内容完全一致。临时清理先校验绝对路径位于私有验证根，不改真实Cargo版本或消费changeset。

验证：完整scripts/check.ps1通过，320项Rust测试成功，fmt、Clippy、release、架构/契约及240文件源码白名单通过。发布模块最终仅重命名流变量以避免自动变量冲突，其合成检查和源码白名单再次通过。官方actionlint固定1.7.12（MIT，校验官方SHA-256）检查全部workflow通过；验证工具与来源材料只留忽略目录。文档本地链接和旧docs引用检查通过，Semifold config sync --check及status通过，未执行ci/version/publish或推送。

实际Windows目标构建与打包通过：当前Cargo版本0.1.0，ZIP10个条目（exe、README、八份Skill文件），独立哈希校验与README字节比较通过；ZIP解出的exe能查询版本和离线列出六类Skill。ZIP不含.agents、docs、NapCat/QQ、研究运行目录、凭据或PDB。没有把0.1.0改为发布计划版本。已存在输出拒绝覆盖，重复验证使用独立合成目录。

分发及CI行为、README/CONTRIBUTING使用patch:chore；纯Agent清单无额外changeset。源码包仍提示既有许可等元数据缺失，仅保留为发布审计事项，不据此增加运行时阻断或自行选择项目许可。

边界：目前无Git远端，未运行GitHub托管runner、创建版本PR或发布Release，不能把actionlint/本地执行当线上通过。仓库需main默认分支及Actions创建PR权限；可选SEMIFOLD_TOKEN供App/PAT触发机器人PR的后续工作流，默认GITHUB_TOKEN受GitHub事件触发限制，需要时手动在release分支运行Quality。仓库规则、令牌和首次远端验收由后续明确授权完成。无真实QQ读取或写入，稳定性验收继续暂缓。
