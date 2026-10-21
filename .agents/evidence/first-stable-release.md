# 首次稳定发布与公开安装验收

文档编号：CHECK-066。状态：端到端通过。日期：2026-10-22。
关联：SPEC-005、TASK-005、TASK-001、TASK-002、CHECK-065。

## 发布结果

- [版本 PR #1](https://github.com/noctisynth/qq-cli/pull/1) 经两项检查通过后 squash 合并，正式提交为 `7ea26b9abff5cbb8558ef011bef285fe94ed6685`。
- [正式发布运行](https://github.com/noctisynth/qq-cli/actions/runs/39677488010) 的质量及 Version or publish 作业全部成功；完整 Rust 测试合计320项。
- [GitHub Release](https://github.com/noctisynth/qq-cli/releases/tag/qq-cli-v0.1.0) 为非草稿、非预发布的 latest，包含 `qq-cli-v0.1.0-windows-x86_64.zip` 与 `SHA256SUMS`。
- [crates.io](https://crates.io/crates/qq-cli/0.1.0) 仅有0.1.0，未撤回，许可MIT OR Apache-2.0；日志确认实际上传和发布成功，未发布alpha。

## 版本与 CI 排查

用户授权在隔离仓库复制当前115个changeset，以0.1.0-alpha.0转正；Semifold0.3.4 status及实际version应用都得到0.1.0，锁文件同步成功。正式准备提交同步Cargo.toml及Cargo.lock，第三方依赖未改变。最终117个changeset由Semifold消费，正式工作区无剩余升版条目。

旧422 head invalid在后续两次版本运行均未复现。上游push没有服务端引用状态回调的诊断缺口已记录，但旧服务端拒绝原因没有原文，不能将过期workflow假说标为确认根因。

确认的独立问题：setup-semifold Windows action将临时install.ps1写入仓库根目录，Semifold全量暂存将其带入首个版本PR。提交feffbda在工作流清除该单个非跟踪文件、拒绝删除同名跟踪文件，并在Semifold前检查工作区干净。重新生成PR仅包含版本、锁文件、changelog及changeset消费，无临时脚本。质量构建统一为分发target，打包复用同一产物。

此前原生启动测试以None unwrap掩盖底层错误；诊断修正后，本地40次定向、98项库测试及后续多次完整远端质量通过。原始偶发原因仍未复现，不标记底层根因已修复，不增加进程创建重试。

## 在线安装与公开包

- 从公开main获取安装脚本，并与审查过的源码比对后，以公开一键调用形式在PowerShell7安装实际latest到隔离目录，使用NoPath且用户PATH保持不变。
- Windows PowerShell5.1再次安装同一目录成功，exe SHA256不变；已安装程序返回 `qq-cli 0.1.0`。
- skill list返回六类任务；message.send Schema查询成功；skill export导出8文件，换行符规范化后与发布源码完全一致。
- 正式ZIP12个条目的范围及SHA256通过发布包检查器。ZIP SHA256为 `057735d922fbb2617031bfb1a61cc4aca828bb02c9eb8b02f71a6e4a6a0c63d3`。
- 实际下载crate的244文件范围、0.1.0版本、双许可和repository元数据核对通过；SHA256与注册表元数据一致。顶层不包含Agent上下文、私人运行材料或第三方QQ/NapCat产物；源码内合法runtime模块保留。
- README已使用真实仓库及稳定安装入口，无未发布占位；Skill包含安装、PATH处理、显式NapCat安装、启动、登录观察及日常操作，不依赖内部开发文档。

本阶段不新增QQ启动或消息实验，不扩大功能验收范围；稳定性继续暂缓，上游已确认问题继续追踪。私有下载和执行记录只保留在忽略目录。
