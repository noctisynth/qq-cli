# Apple Silicon 已连接 TCP 归属与受检传输

文档编号：CHECK-075。状态：Native synthetic verified。日期：2026-10-24。
关联：SPEC-008、DESIGN-002、TASK-006。起点：d652af0。

## 实现与核对

只对已核对的 Pinned 服务进程使用 PROC_PIDLISTFDS / PROC_PIDFDSOCKETINFO，限8192个描述符，不扫描全系统进程或输出其它连接信息。
原创 socket.c 使用 Apple SDK libproc.h/sys/proc_info.h 的真实结构读取 TCP ESTABLISHED 端点，向 Rust 返回窄整数/字节结构；C/Rust 分别静态检查其大小和地址偏移。依赖已有锁定 cc 1.8.0，仅 macOS arm64 编译该适配，CLI/业务逻辑保持 Rust。
已连接客户端四元组反向匹配唯一服务端描述符，再核对原描述符的 socket 标识/代次和完整进程身份。等待 accept 有总截止/2秒/200次限制，查询异常及重复匹配立即拒绝；检查期间不发送协议字节。
OneBot/WebUI HTTP 与 WS 复用原协议实现，平台 guard 只负责原 TCP 流验证及返回后身份复核。Windows Job/成员检查保持在 Windows guard；macOS 受检接口仍须由后续认证运行日志提供 Pinned。

## 验证结果

- 新增7项测试（含原创子进程入口）：IPv4/IPv6原fd保留、延迟accept、错误进程/截止、OneBot和WebUI HTTP、WS握手、错误进程无法接收HTTP/WS凭据、只有监听而未accept的拒绝。
- 拒绝场景的服务端读取为0字节；成功场景校验合成Bearer或独立WebUI hash。所有数据为合成值，不涉及QQ会话。
- 初版合成服务在macOS accept后继承非阻塞状态而立即读失败；显式切回阻塞并保留I/O截止后通过，不改产品验证逻辑。
- scripts/check-macos.sh：fmt、Clippy、219项测试和arm64 release构建通过。
- 源码包252项，白名单、全部源码及原创C适配/build.rs包含检查通过；内部上下文/研究材料不包含。
- 同步PowerShell原生边界和包白名单；检查四处unsafe声明及macOS精确声明正则。修复前阶段rustfmt换行注释导致的匹配问题。本机未执行完整PowerShell/Windows原生回归。
- Semifold status通过；阶段变更集macos-owned-connections。

## 限制

此证据不代表CLI的macOS安装/启动、签名运行恢复、停止或多实例已完成。其它描述符瞬时消失也可能造成严格拒绝；没有自动放宽查询或重发业务请求。
观测证明该时点的指定进程持有服务端socket，不能禁止之后的恶意转移，亦不证明进程树隔离、GUI兼容或账号安全。
最低系统版本与Windows原生回归仍按TASK-006处理。
