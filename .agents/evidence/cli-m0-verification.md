# Rust CLI M0 离线验收

文档编号：CHECK-008。状态：Offline verified。更新：2026-10-17。
关联：SPEC-004、PLAN-001、RFC-0002、TASK-002。无新的 QQ/NapCat 请求或消息操作。

## 工程和迁移

- 单 Cargo binary crate，edition 2024，版本 0.1.0；工具链与当前最低支持版本固定 Rust 1.98.1。
- 目标为 Windows x86_64 MSVC；rustfmt、Clippy 和链接过程均实际通过，不宣称其他目标已支持。
- Cargo.lock 锁定 clap 4.6.7、serde 1.0.229、serde_json 1.0.154、reqwest 0.13.5、sha2 0.10.9 及传递依赖。
- cargo tree 核对 reqwest 仅有 blocking/json 顶层 features，无 TLS/压缩；未构造客户端，代理/重试策略属于 M1。
- Semifold resolver 已改为 rust，读取 Cargo.toml 的 0.1.0，旧 package.json 已删除；config sync --check 通过。
- 工具链安装、依赖锁定和缓存准备使用网络；其后的检查均使用固定工具链、缓存及 --offline --locked，不混称全过程无网络。

## 行为与测试

全部业务命令仅解析参数，返回单行 JSON NOT_IMPLEMENTED、outcome=not_attempted、退出码 6。
无效参数返回 INVALID_ARGUMENTS、退出码 2，不透传 Clap 的原始诊断；help/version 返回文本和退出码 0。
invocationId 按本次命令生成，成功解析 send 时单独保留调用方 requestId；不输出消息正文、文件名或 token。

8 个 CLI 子进程测试通过：

1. 根及 message 子命令帮助、版本离线可用。
2. 缺失/未知命令、非法标识及历史数量越界返回 JSON，错误不回显合成秘密值。
3. doctor/status/self/get/history 全部明确未实现，大整数和负消息 ID 可被解析为字符串。
4. send 的 text、text-file、stdin 三种来源均可解析；缺失文件不打开，保持 stdin 打开时仍及时退出。
5. send 要求目标、请求键和恰好一种文本来源，拒绝来源冲突、非法/过长请求键与空 text。
6. --config 可位于不同命令层级；缺失配置文件不被打开。
7. 设置本地假后端与合成 token 后，全部业务命令仍未建立连接。
8. 两次子进程调用的 invocationId 不同。

固定工具链下 fmt、Clippy（-D warnings）、全部测试与 release 构建通过；release 版本及 status 错误包装另行核对。
检查脚本位于 scripts；不会启动 QQ 或调用实验探针。

## 源码包检查与边界

源码包白名单检查通过，共 14 项，包含 Cargo 自动生成的元数据，未包含 Agent 上下文、上游、运行材料或凭据。
初次未锚定的 README 模式曾匹配研究目录同名文件；已改为根锚定 include，并增加可重复执行的清单检查。
仅检查清单，未发布或上传包。Cargo 提示缺少许可/仓库等发布元数据；保持 publish=false，未自行指定项目许可。

尚未实现配置读取、HTTP 请求、账号门禁、消息读取、发送、发送记录或实时事件；M1 至 M4 均未勾选。
这些测试不证明真实会话或账号安全；历史单次实验授权未被重用。
收尾复核：30 个文档/数据编号唯一且引用完整，仓库内文档链接及公共源码/文档隐私扫描通过；Git diff 无空白错误。
Semifold 配置同步通过，版本仍为 0.1.0；新增 M0 功能 changeset，未消费条目、提升版本、发布或推送。
