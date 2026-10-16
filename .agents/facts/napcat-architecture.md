# NapCat 架构研究基线

文档编号：FACT-001。状态：Source-observed。更新：2026-10-16。
来源：EVIDENCE-001，承接建项前本次对话中的公开源码阅读。
后续固定版本及真实附件核对见 FACT-002、DATA-001、CHECK-002；运行行为仍未验证。

## 已观察内容

- Windows Shell 的 launcher.bat 定位已安装 QQ，传递程序路径与 NapCatWinBootHook.dll 给启动器。
- 公开 Windows Boot 源码创建暂停的新进程，加载 DLL 后恢复运行；不是现有 PID 的接管接口。
- Shell 的 qqnt.json 将入口指向 loadNapCat.js；加载器导入 napcat.mjs。
- Shell 自行加载 wrapper.node 并初始化引擎、登录服务和会话；复用安装文件不等于共享现有会话。
- Framework 的 napcat.cjs 在原生模块加载处接入，并通过代理向 QQ 返回共享会话和登录服务。
- nativeLoader.cjs 接收 session、loginService 和初始化回调并调用 NCoreInitFramework；
  此函数签名没有 LiteLoader 参数。这不足以证明发布包支持独立加载或与当前 QQ 兼容。
- 消息 API 通过内部消息服务执行，不依赖输入框点击，因此具备避开桌面输入竞争的架构基础。

## 推断与未知

独立启动加载器安装接入逻辑、继续官方入口、共享会话，再启动 API，在架构上有依据，尚无原型。
不能仅把 Shell 入口和官方入口同时运行就认为共享会话；重复初始化可能冲突。
不能保证双进程同账号共存，也不能保证加载器与发布产物接口、初始化时序或新 QQ 兼容。
不依赖 LiteLoader 不意味着不注入；源码中存在 Hook 和原生处理逻辑。
风险概率、协议合规性、封号情况、性能和全历史完整性均未做验证，不提供肯定结论。

本页早期 main 来源仅作背景，本轮 Framework 审计以 FACT-002 固定基线为准。
Shell 与另一 Windows Boot 仓库未在本轮和发布包完整匹配，不把其源码套用到 Framework 原生加载器。
重要修正：实际 Framework 包已有不经 LiteLoader 脚本的原生入口，自有加载器不是已确定前提。
