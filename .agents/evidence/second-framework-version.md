# 第二个固定 Framework 版本

文档编号：CHECK-025。状态：静态来源/配置核对及有界离线验收。更新：2026-10-19。
关联：DATA-001、FACT-002、CHECK-015、CHECK-016、CHECK-024、SPEC-004、RFC-0003、TASK-002。

## 固定来源与实际包

检索2026-10-19。[官方v4.18.37发布页](https://github.com/NapNeko/NapCatQQ/releases/tag/v4.18.37)及[发布API](https://api.github.com/repos/NapNeko/NapCatQQ/releases/tags/v4.18.37)均为可变来源，release ID408592169、Framework asset ID626888542，immutable为false；不能仅凭URL复现基线。注释tag对象bf4080f24bc0e9022f2910e03dfcdd21a4c201fd解析为提交ff9a829e29038eeec3e23b062ec90c838fc9efa3。研究JSON、Git对象及包仅留忽略目录，不提交上游代码或二进制。

实际下载[固定版本Framework附件](https://github.com/NapNeko/NapCatQQ/releases/download/v4.18.37/NapCat.Framework.zip)，长度36,723,625字节，SHA-256为59d9538c1f1b66ff1269c362069fcd7d1b1d6793c35255ae80e699b91e37401c，与官方附件digest一致。ZIP有680个文件；基础名字核对无重复/大小写冲突、绝对/父目录/反斜杠/冒号入口。此静态名字核对不替代Rust完整归档安全规则。

| 必需成员 | SHA-256 |
|---|---|
| napimain.exe | 116a7da5a0887244a4946472062a0cb0b866e353924808b659ff7d109b5ffe69 |
| napiloader.dll | f7dba30fc165ac013e4b9114df851d4c7dbb7853d05175df5050c7886e9d5311 |
| nativeLoader.cjs | 98e12440f27444c686de8087bd69381bb3f3e7e567870a90507f5976e0b3e38f |
| napcat.cjs | 0bacde790aa4081b3a2d3b18faae3dee39f43b44d9fe1f622602fd2604a74887 |
| napcat.mjs | dcc02d22e4ec2390c0bd1a082e44ac4bd37a02b69fddbc0846cdebcf3023f9ac |
| native/napi2native/napi2native.win32.x64.node | 46780f9cc420a840318179b26cbb551357d2ae986dd8df960ef1a49656181fcc |
| native/packet/MoeHoo.win32.x64.node | fb62bff4d093b892bbe5c5dc4197a226881689e43f6d438a66895fa7cdbb90f1 |

与DATA-001基线包重新核对，615个同名文件摘要相同，63个删除/63个新增（前端带内容摘要的文件名），2个同名内容变化为napcat.mjs及static/index.html。七个必需成员中只有mjs不同；仍分别固定完整包及mjs摘要，不混装。源码LICENSE与README blob分别为a79f4f97db1bab5ada62d8844aaa33291ef2e604、67c0bcf7519363ee00b49d364f3b5ac20ec78692，两提交相同；包根仍无LICENSE或README。FACT-002未决许可边界保持，不把可下载解释为获许可。

## 配置与只读控制核对

[固定源码树](https://github.com/NapNeko/NapCatQQ/tree/ff9a829e29038eeec3e23b062ec90c838fc9efa3)与v4.18.38对应对象核对：Framework入口blob29604e7789ec49346a38e762b98e68454bf27f5a，OneBot配置config.ts为71e1edcf5b3372c70817936a9365b5ece680702a、配置index.ts为fe84992b8883ae577166678ea3f52a0a5e5b0751，WebUI入口01d83af1d93c52884ce73834ab1b1c2048ad50d7，Auth.ts为05aafd79b0d2baa5effacb48cf885492435bc854，均相同。

CHECK-016中的鉴权签发/中间件、路由挂载、Data/类型、响应封装及状态常量blob也相同。QQLogin.ts和其路由整体有变化，不能声称整个模块相同；逐项核对QQCheckLoginStatusHandler，提取处理函数的UTF-8文本SHA-256两版均为c9c4ea6046fb95e3ee341381e1a5fab4952c31c7fd1ce8b6167a20a8530bff17，其POST路由不变。已检查差异涉及设备管理等其他接口，不扩展CLI调用范围。发布mjs差异静态复核与这些观察一致，不证明实际服务已启动或已登录。

因此目录可为两版复用现有配置模板/只读控制协议，仍生成每run独立秘密和配置。目录数据与选择逻辑拆分，默认仍v4.18.38，不接受latest、任意版本或外部信任清单；相同加载器摘要不产生或继承真实QQ兼容见证。

## 验证与未完成

7项归档/配置测试在原生权限环境通过，覆盖两版规范化选择、独立包/mjs摘要、两版配置字段和凭据域隔离，未知版本拒绝保持。首次受限环境运行中，两项原归档准备测试返回ARCHIVE_UNSAFE而未到预期阶段；同代码原生复核7项通过。未修改归档、私有ACL或生产门禁，不据此确定底层失败原因。随后完整280项、fmt/全目标clippy/release、架构及202文件源码包通过；Semifold配置/发布计划通过，不消费changeset。实际包和七个成员摘要与目录源码再次核对一致。

没有执行下载的EXE/DLL/JS，没有QQ启动、登录、账号API或焦点实验；未通过CLI安装真实上游包，真实下载适配器/安装验收、公开更新回滚及真实兼容仍待。本次扩展既有固定目录任务的版本覆盖，不增加业务完成计数，66/99、待完成33保持。
