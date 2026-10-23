# 自动化与发布清单

文档编号：TASK-005。状态：首次稳定发布端到端验收完成。更新：2026-10-24。
需求：RFC-0007。技术：SPEC-002/005。

- [x] 内部架构/设计移入 .agents，并从分发范围排除；CHECK-064。
- [x] Windows 复用质量 workflow、PR Semifold 状态、main Semifold CD 和资产准备；CHECK-065。
- [x] 完整离线质量、workflow 语法、打包/校验及源码范围；CHECK-065/066。
- [x] PS5.1/PS7 安装器的合成验证及真实 latest 安装/重复安装；CHECK-066。
- [x] MIT OR Apache-2.0、双许可证分发、crates.io 命令和受信令牌注入；CHECK-066。
- [x] 首次稳定 0.1.0 的真实 CI、版本 PR、crates.io/GitHub Release 和在线安装；CHECK-066。

后续 macOS CI/分发由 TASK-006 跟踪。此清单不证明完整业务稳定性、所有升级场景或上游授权已解决。
setup 临时文件、初次版本流程和旧 CI 问题的过程保留 CHECK-066，不在当前清单复制流水。
