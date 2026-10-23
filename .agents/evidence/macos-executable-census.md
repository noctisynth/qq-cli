# macOS 精确可执行文件候选发现

文档编号：CHECK-077。状态：Native synthetic verified。日期：2026-10-24。
关联：SPEC-008、TASK-006。起点：1117113。

## 方法与边界

核对本机 Apple SDK libproc.h：proc_listpidspath 返回精确文件引用的PID列表，长度单位为字节；不是仅可执行映像列表。sys/event.h 明确 NOTE_TRACK/NOTE_CHILD 自10.5起不受支持，不能用 kqueue 自动追踪整个fork树。
独立原创 C 探针创建专用可执行文件副本，子进程 setsid 后父进程退出：仍返回一个实际映像匹配；额外打开文件的读取者也被系统返回；子进程自然退出后匹配归零。探针保留在本地临时目录，未操作 QQ。
生产候选观察通过原有 macos_process unsafe 边界调用原创 census.c，限制4096项并拒绝饱和结果。固定文件句柄和元数据前后复核，Pinned 二次过滤实际路径；元数据不替代内容摘要，候选不证明祖先或完整树。

## 仓库测试

新增两项集成测试覆盖：读取者排除、专用副本进程识别、退出后候选消失、父退出且子进程使用独立进程组后仍可发现、路径替换拒绝、符号链接/硬链接/权限变化拒绝及截止时间。
受限沙箱禁止原有进程观察，原有及新增测试均受影响；在授权的原生合成测试环境重跑，五项进程测试通过。scripts/check-macos.sh 的 fmt、Clippy、226项测试及arm64 release构建通过。源码包257项白名单及全部源码覆盖检查通过，git diff --check 与 Semifold status通过；变更集为macos-executable-census。Windows/PowerShell原生回归未在本机执行。
本阶段未验证QQ权限、sandbox执行白名单、fork/exec全树收敛、签名运行日志、实际CLI停止或最低系统版本。
