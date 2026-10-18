# 固定 Framework 预登录监听与保留候选核对

文档编号：CHECK-015。状态：固定源码及发布包观察，非真实进程验收。更新：2026-10-18。
关联：FACT-001、DATA-001、RFC-0003、SPEC-004、TASK-002、CHECK-013。

## 固定基线

NapCat v4.18.38，提交bbd0d2aca3d470c20b33b1d4f95123335633bf26。只读检查既有审计仓库的固定Git对象，缺失对象从原公开仓库取得；未执行上游代码、启动QQ或访问账号API。
已有发布包napcat.mjs的SHA-256再次匹配DATA-001：f79260cb7e6c4b579a4346ffd39a91a33ea73f9b7cd44d9ce204ab471096601a。研究副本仅在忽略目录。

| 固定源码 | Git blob | 观察 |
|---|---|---|
| [Framework入口](https://github.com/NapNeko/NapCatQQ/blob/bbd0d2aca3d470c20b33b1d4f95123335633bf26/packages/napcat-framework/napcat.ts#L88) | 29604e7789ec49346a38e762b98e68454bf27f5a | NCoreInitFramework中先调用InitWebUi，随后等待登录Promise；成功后创建协议适配器 |
| [WebUI入口](https://github.com/NapNeko/NapCatQQ/blob/bbd0d2aca3d470c20b33b1d4f95123335633bf26/packages/napcat-webui-backend/index.ts#L113) | 01d83af1d93c52884ce73834ab1b1c2048ad50d7 | InitWebUi在本模块创建HTTP/HTTPS server，直接listen；配置禁用、初始化失败等可能不监听 |
| [适配器管理](https://github.com/NapNeko/NapCatQQ/blob/bbd0d2aca3d470c20b33b1d4f95123335633bf26/packages/napcat-adapter/index.ts#L89) | e9ed47ee470debfa8e86ddffaf12337b360e0a1e | initAdapters直接构造OneBot适配器并初始化 |
| [OneBot入口](https://github.com/NapNeko/NapCatQQ/blob/bbd0d2aca3d470c20b33b1d4f95123335633bf26/packages/napcat-onebot/index.ts) | 838af29b6b4690bf43d2f5d3e9e0f13fecfe4007 | 登录后适配器配置与网络适配器构造链 |
| [OneBot HTTP](https://github.com/NapNeko/NapCatQQ/blob/bbd0d2aca3d470c20b33b1d4f95123335633bf26/packages/napcat-onebot/network/http-server.ts#L70) | a8277eabfc6416271ebb948154b6c47cf4f95f6e | initializeServer直接创建HTTP server并在配置host/port监听 |

发布包交叉观察：已核对摘要的napcat.mjs包含同名Framework入口、WebUI入口与适配器类。Framework段中InitWebUi调用紧接登录Promise，登录后才构造NapCatAdapterManager；不是仅凭TS源码推定打包入口存在。

## 可得结论与边界

固定源码及发布包支持“先尝试启动WebUI，再等待登录，随后初始化OneBot”的时序。这里的启动调用是异步调用且错误会被捕获，不表示WebUI在进入登录等待前已经成功listen，也不承诺固定时间内成功。

同一Framework调用链直接调用WebUI和OneBot网络构造，未在这条所检构造链看到将两类监听移交其他进程的步骤；据此可推断同Node进程候选有架构依据。它不是实际QQ进程身份测量，也不排除上游其他初始化、更新、重启或QQ行为改变宿主。真实完整元组仍须验证控制端点归属、登录前后同完整PID/创建时间/映像、Job保留及窗口/API共存；reviewed_witnesses保持为空。

## 端口漂移约束

[InitPort](https://github.com/NapNeko/NapCatQQ/blob/bbd0d2aca3d470c20b33b1d4f95123335633bf26/packages/napcat-webui-backend/index.ts#L70)会处理首选端口变量，首选失败仍可回到配置端口；[tryUsePort](https://github.com/NapNeko/NapCatQQ/blob/bbd0d2aca3d470c20b33b1d4f95123335633bf26/packages/napcat-webui-backend/index.ts#L551)对EADDRINUSE可递增端口，MAX_PORT_TRY为100，探测与实际listen之间仍有竞争窗口。

qq-cli不跟随该漂移，不扫描端口、不从日志提取可能含token的URL，也不对未验证监听发送认证。仅签名预期端点可作有界零字节归属检查；未命中、归属不符或控制/API身份变化必须保留未知/未就绪结果。端口预留释放到上游实际监听不能原子交接，完整新建流程仍须处理这一限制，不增加force或自动终止用户会话。

## 验收状态

五份研究副本均用Git对象哈希与固定提交路径逐一比对通过；文档编号唯一性、编号引用、公共上下文隐私、57/90任务计数及Git差异格式检查通过。本次没有Rust变更，不重复运行此前201项离线回归；纯内部上下文不新增changeset。

已完成本次静态时序、构造链、固定blob及发布包摘要交叉核对；CHECK-013的原创父子/延迟API实验仅验证保留候选原语。未获得真实预登录/登录后PID证据，未冻结RFC-0003底层机制，未解开许可/兼容门禁，未执行真实会话操作。TASK-002整体57/90不变。
