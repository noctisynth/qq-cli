# macOS 专用副本入口配置计划

文档编号：CHECK-088。状态：Synthetic package plan verified。日期：2026-10-24。
关联：SPEC-008、TASK-006。起点：b494557。

## 源码观察边界

只读复查CHECK-072保留的官方7.0.2-53644原package元数据：main为./application.asar/app_launcher/index.js，platform为darwin，eleArch为x64。保留原package字节的SHA-256为ed893137a817d8442107c548e10362686c8713618fda796eb625cad6ebbd99d9；这是本地研究材料观察，不替代部署时从固定DMG重新认证。CHECK-072的Mach-O/实际运行arm64证据与元数据字段不同，不能按eleArch拒绝或确认原生架构。
外部app_launcher/index.js与launcher.js只转交major.node中的internal_index/internal_launcher；本次静态观察不足以证明原型setImmediate修正launcher.installPathPkgJson.main的时序适用于未知版本。没有启动QQ或调用原生模块，也未复制上游源码到公共仓库。

## 实现与合成测试

PackageEdit绑定调用者提供的已认证原摘要、预期版本/构建和固定原main，拒绝重复顶层字段及非CommonJS模式，再核验原创适配资源。只改main字段，保留其它字段语义，返回改写字节、前后摘要和固定专用副本目标。格式重新序列化，不承诺其它字节逐字节相同；未实际写package或重签副本。
新增2项Rust合成测试：字段保持/字面入口/前后摘要/源与结果复核/不写文件；错误摘要、版本、名称、平台、已改入口、模块类型、构建号、重复字段及已发布资源篡改拒绝。测试使用原创合成JSON，没有上游包复制。
实际安装层仍须认证原摘要、Mach-O架构、专用副本与发布边界，接合签名/回滚和运行期入口兼容。调用者自行对任意输入计算摘要不构成可信来源。

完整macOS门禁通过：22项Node测试、fmt、Clippy、257项Rust测试（2项忽略）及arm64 release构建。源码包274项白名单及全部src覆盖、git diff --check与Semifold status通过。阶段变更集macos-copy-package-plan。
