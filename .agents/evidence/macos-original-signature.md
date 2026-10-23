# macOS 官方源静态签名验证

文档编号：CHECK-090。状态：Synthetic rejection and installed-source verification passed。日期：2026-10-24。
关联：SPEC-008、TASK-006。起点：3b4d2ad。

新增安全Rust接口组合SecStaticCode严格资源、嵌套代码、全架构和禁网络检查。分别固定Developer ID/商店签名要求，不接受自签/ad-hoc。直接依赖core-foundation固定0.10.1，已有lock版本未升级；未新增unsafe。

新增3项普通测试：无效/别名路径与已截止拒绝；复制原创测试程序到临时合成.app、以com.tencent.qq标识ad-hoc签名且系统strict验证通过，再确认官方源检查拒绝；纯结构可解析的合成QQ应用不能通过inspect_original。未复制上游代码到测试或修改QQ，未查询Keychain条目。

## 原生只读观察

已安装QQ的designated requirement包含Apple商店与Developer ID两个分支。只读codesign --verify --deep --strict成功；强制腾讯Developer ID叶证书OU分支失败，Apple商店OID+identifier+Apple anchor分支成功。沙箱内最初返回NOT_TRUSTED，允许访问系统信任服务后获得上述结果；不能把沙箱服务不可用解释为QQ包损坏。
Rust inspect_original首次在30秒预算下返回DEADLINE，未接受该结果。系统签名调用同步不可取消，调用前后截止检查不构成硬墙钟上限；后续产品编排必须补隔离执行和超时收敛。

禁止网络验证不提供实时吊销或在线公证保证；嵌套签名检查不自动认证下载DMG或授予修改/启动普通QQ的权限。当前固定DMG安装路线仍需独立审计，商店签名分类不等于其部署路线已支持。

同一Rust组合接口以120秒验证预算重测成功：origin=AppleAppStore、version=7.0.2-53644、architecture=aarch64；指纹5fa6d9836ad144f7ebbc62434182fdb54cf06cbaa6e49b045a9600d6db8fb350与CHECK-089一致。没有真实QQ启动、会话读取或签名修改。

完整macOS门禁通过：22项Node测试、fmt、Clippy、264项Rust测试（2项忽略）及arm64 release构建。源码包279项白名单及全部src覆盖、git diff --check与Semifold status通过；阶段变更集macos-original-signature。
