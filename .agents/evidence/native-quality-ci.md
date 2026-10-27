# 双平台原生质量门禁修复

文档编号：CHECK-137。日期：2026-10-28。状态：双平台原生离线门禁、构建、归档和发布资产汇总通过；版本PR未合并，线上发布另列。
关联：RFC-0008、SPEC-005/008、TASK-006；真实 QQ 功能验收继续以 CHECK-136 的固定组合为准。

## 原因与修复范围

| 失败 | 实际证据与修复 |
|---|---|
| Windows Clippy | 原生编译发现生产路径未使用的 save 与 Started::stale；将测试辅助函数限定为 cfg(test)，移除无用方法。macOS 的 all-targets 不编译 Windows 条件分支。 |
| 系统镜像报告 | macOS 15.7.9 的无关预挂载系统镜像没有 image-encrypted；报告按所有镜像强制字段，导致托管挂载测试提前拒绝。改为可缺字段，仍要求被选中的受管镜像明确 Some(false)，未放宽目标所有权、设备、只读或代次核验。 |
| plist 转换 | 本机另行复现 plutil 转 JSON 不能表示 plist Data。使用固定 MIT 许可 plist 1.8.0 安全 Rust 直接解析所需字段，忽略无关 Data/Date，增加二进制媒体元数据回归；此缺陷不冒充原始托管报告失败的全部根因。 |
| Windows TLS 夹具 | Schannel 返回 TRUST_E_CERT_SIGNATURE（0x80096004）；合成 CA 和叶证书原先使用同名 DN。将两者 CN 分开，叶证书补充 authority key identifier；仅改变客户端自信任的短期测试证书，生产 TLS 校验和系统信任未变化。 |
| macOS 复制夹具 | 同一原创合成文件在 macOS 15 上经 ditto 后缺失零长度扩展属性，经 fcopyfile 则完整保留；本机 macOS 27 的 ditto 不丢失该属性。将此用例改为调用受控 copy_to_new，并显式验证空属性存在、删除后仍拒绝副本；无需改动生产复制代码或放宽属性比较。 |
| Windows CLI 夹具时序 | 后续全量复验暴露 send/reply 合成服务在实例准备完成前计算 5 秒等待预算；此前同套测试通过不能保证不存在竞争。增加显式 start 就绪信号，保持请求等待和读写截止不变；新增故意准备 6 秒的跨平台回归。 |
| macOS 释放夹具时序 | 原生复验中首次 release 返回 Unknown，立即 Released 的测试断言失败；旧夹具未保留回调效果错误，不能把该次 Unknown 唯一归因于异步观察。弹出返回值本来也不等于设备消失证明。正例改为检查实际 unmount/eject 成功，再在原 60 秒预算内用相同签名日志只读恢复到 Released；恢复回调一旦被重派就 panic。负例继续断言 Unknown，生产释放算法与守卫不变。 |

修复提交：48bbb5e、6c422e2、aeac394、2baa50f；c762d7e、1a5d4aa、af8b444 用于限定合成诊断，工作流临时诊断现已全部移除。没有操作真实 QQ、系统 Keychain 或无关应用。

## 托管证据

- 原失败：[40550467874](https://github.com/noctisynth/qq-cli/actions/runs/40550467874)。
- 诊断：[40559995811](https://github.com/noctisynth/qq-cli/actions/runs/40559995811)，确认缺少 image-encrypted 及 Windows 原生 TLS 错误。
- 中间复验：[40561598083](https://github.com/noctisynth/qq-cli/actions/runs/40561598083)，Windows 完整质量门禁和归档通过；macOS 挂载测试通过，后续复制夹具失败。
- 合成复制诊断：[40564031945](https://github.com/noctisynth/qq-cli/actions/runs/40564031945)，原生诊断步骤成功，随后主动取消重复旧代码全量检查；该轮不计作完整通过。
- 后续复验：[40565731212](https://github.com/noctisynth/qq-cli/actions/runs/40565731212)，暴露 Windows CLI 夹具准备和 macOS 释放观察的时序假设，不计作通过。
- 最终复验：[40567972559](https://github.com/noctisynth/qq-cli/actions/runs/40567972559)，源码 2baa50f466b3ed7e8529f9fe2cdcebbfc53b6208；Windows 与 macOS 质量 job、平台打包及归档上传均成功。

## 本次托管结果

固定 Rust 1.98.1，Windows 2025 原生 x86_64-pc-windows-msvc 与 macos-15 原生 aarch64-apple-darwin；未使用交叉编译结果代替原生执行。

| 范围 | 实际结果 |
|---|---|
| Windows Rust | 62 个套件汇总，330 passed、0 failed、0 ignored；包含发送、回复、TLS 与延迟就绪回归 |
| macOS Rust | 62 个套件汇总，508 passed、0 failed、2 ignored；忽略项为既有原生Keychain测试及其子入口，未执行且不计入离线通过 |
| macOS 配套门禁 | Python 15 项、Node 28 项、fmt、全目标 Clippy、完整离线测试、独立装配宿主 Complete/Unknown 恢复和不重放均通过 |
| 产物 | 两平台 release 构建、版本检查、平台打包及同 SHA 归档上传通过，发布job验证两个ZIP及公共SHA256SUMS；分发仅含原创CLI/文档/许可证/Skill |

同次workflow整体success，发布job的资产汇总、干净工作区检查和Semifold CI均成功；[版本PR #2](https://github.com/noctisynth/qq-cli/pull/2)保持open，未手动合并、打tag或发布。

临时工作流诊断已清除；生产运行所有权、TLS 校验、目标镜像加密声明、属性比较和未知效果不重放均保持。真实 QQ 固定组合结论仍限 CHECK-136，不从本次合成 CI 推导其它系统上的真实 QQ 可用性。

## 验证边界与遗漏原因

原本地验收以 macOS 27.0 和固定 QQ/Framework 为主，完整离线门禁和真实功能通过不能代替 Windows 或 macOS 15 原生执行。此次暴露的是交付前平台矩阵覆盖不足，既有合成夹具也未覆盖系统报告字段缺省及旧版 ditto 行为。

本地定向复验包括 39 项镜像生命周期、报告解析、二进制媒体元数据、4 项原生 TLS、19 项应用树集成测试、7 项释放/清理测试及 1 项延迟 HTTP 就绪回归，以及 fmt 和全目标 Clippy -D warnings；托管汇总以最终日志为准，不重复计算重跑结果。

本次只验证原创合成资源、离线 CLI 契约、原生系统能力和分发流程；不运行真实 QQ，不把托管 macOS 15 结果扩展为 QQ 的最低系统版本、全部上游扩展或长期稳定性承诺。线上安装和正式发布仍另行跟踪。Semifold status 保持 10 份 changeset，不手工消费、升版或发布。
