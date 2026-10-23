# Apple Silicon Framework 运行适配方案

文档编号：SPEC-008。状态：接入原型已验证；Rust 平台实现待完成。更新：2026-10-24。
需求：RFC-0008。事实：FACT-005；验证：CHECK-072；任务：TASK-006。

## 装配与签名

以固定官方 DMG 建立工具拥有的专用 QQ 副本，保留原安装；不复制真实 QQ 私人数据。
副本 package.json.main 指向外部薄适配入口，继续原 QQ application.asar/app_launcher/index.js。
只对专用副本本地 ad-hoc 重签名。已验证的原生 hook 需要 allow-jit、disable-library-validation，
主进程另需 allow-unsigned-executable-memory；不加入 disable-executable-page-protection，不关闭 Gatekeeper/SIP。
这是验证组合的配置，不是其它 QQ/系统版本的通用白名单；生产安装需重复验证来源、完整性和权限范围。

移除副本的 App Sandbox/共享组权益后，实验使用外部 sandbox 强制目录与进程隔离。
产品必须实现并验证等价的所有权和隔离边界，不能因副本可运行就省略隔离；实验脚本不直接充当安装器。

## 真实会话转交

适配在官方入口前安装精确 wrapper.node 注册观察，仅处理副本的固定模块路径，其他模块原样转发。
原 QQ 自己调用 getNTWrapperSession("nt_1") 时，向原版 nativeLoader.cjs 的 initializeNapCat 转交取得的原生对象和登录服务。
不调用不存在的 session.create，不创建另一 session，不运行 Shell 初始化。

原生 init/getter 属性不可写；通过可替换的模块导出和转发视图接入，保留原 receiver、参数、返回、异常和其它方法。
观察官方 init 的真实 onOpentelemetryInit，先执行原回调，再在 is_init === true 且代次匹配时释放 Framework 初始回调。
先注册/后通知或先通知/后注册都必须正确；拒绝过期代次及 truthy 非布尔成功，初始就绪只释放一次。
该原型不是切号/重连状态机；后续恢复需要独立设计和验收。

当原生 getNTUserDataInfoConfig 返回空值时，外部适配回退到受控 userData；非空原生结果原样保留。
每次精确模块注册均应用该兼容，不改上游文件；生产使用前仍须检查最终数据路径属于实例。

## 实例目录和端点

使用结构正确的私有容器布局：<instance>/profile-home/Library/Containers/com.tencent.qq/Data。
QQ 由 home 推导的 Group Containers 必须仍落在同一实例根下。
Electron home/appData/userData、documents/downloads/desktop/media、temp/logs 逐项显式设置到受控目录。
不能用扩大到原用户目录的读写权限解决路径失败。符号链接/祖先解析、旧路径迁移和并发写入需产品级检查。

原版默认配置名为 onebot11.json，账号配置为 onebot11_<account>.json；不能写成 onebot.json。
新实例不预填账号，不覆盖已有账号配置。业务 HTTP/WS、管理端点及凭据分离，回环监听，关闭非需求插件/CORS/调试。
上游日志留私有目录；不能用日志中的 token/账号/二维码作为公共状态输出。

## Rust 平台接口

通用编排依赖平台能力：准备启动、验证进程身份、验证同一连接归属、观察退出、停止已拥有运行。
macOS 身份不能复用 Windows FILETIME/Job 的编码；平台标记、旧记录读取/拒绝和迁移策略先定义再实现。
libproc、kqueue、Keychain、进程组或保管机制是候选，尚未因原型实验被证明足以满足所有权契约。
要覆盖 PID 重用、父退出、子进程脱离、端口转移、凭据恢复、停止前重核及失败清理。

保留同一 socket 的归属核验后再发送认证，不能把一次 lsof 监听查询或 URL/token 当作产品受检连接。
安装发现、DMG/app 处理、签名、兼容诊断与生命周期接合后，再复用 OneBot/领域层并补 arm64 CI/安装分发。

## 验收界限

原型验证启动、GUI、真实初始化、身份一致及 10 个读取接口。macOS Rust CLI 尚未接合。
私聊历史、写入、媒体、事件/恢复、多实例、更新及长期稳定性分别登记 TASK-006，不能用本原型结果勾选。
不支持 Intel/Rosetta；最低 macOS 版本未由单机实验确定。OCR/WebUI 终端的上游限制见 FACT-005。
