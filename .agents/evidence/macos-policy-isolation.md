# macOS 签名权益检查隔离

文档编号：CHECK-106。日期：2026-10-24。状态：实际CLI工作进程正反例及完整macOS门禁通过。
关联：SPEC-008、TASK-006。起点：c0be38a。

安装层新增只读Policy工作进程并注册main私有分派。父端调用当前宿主，不接受可替代程序；固定main/electron/native及有界预算，清空环境、不解析输出。复用native层来源检查的进程管理与独立父EOF/截止看门狗，保持native无反向Policy依赖。专用95表示权益通过，来源91/92不能混用。check_worker在效果前、轮询及结果后复核guard与截止；取消有界回收原Child，不确定wait不发信号，已消费Child不重复Drop终止。

11类原创签名夹具逐个经实际CLI私有入口校验，与同步Policy结果一致：Main/Electron/Native及缺省Native权益通过；缺权益、额外sandbox、false/字符串权益、缺runtime、代码篡改及角色不符拒绝。仅签名原创测试程序副本，不执行这些副本。协议覆盖缺参数、未知角色、零/超限预算和额外参数，全部静默拒绝；父端guard失败/已截止在派发前返回。初次测试使用错误Cargo二进制名导致编译失败，改为项目实际qq-cli后二者通过，不将该失败解释为系统能力差异。

共用进程管理保留原来源检查成功/失败/未知退出码、截止、原进程回收和父EOF看门狗回归，新增guard丢失回收反例。Inventory.sign_copy_with_isolated_checks已组合逐目标/最终权益隔离与父端整树守卫；合成全事务仍由同步对照覆盖，新组合没有冒称真实QQ端到端验证。没有新增unsafe或依赖。

codesign写入进程的父死亡监管、文件系统持久化/总预算和恢复日志继续待办，未借只读worker宣称完整签名事务。没有真实QQ/账号/Keychain操作；Windows原生/PowerShell门禁未执行。仅本地提交，不推送或发布。

最终门禁通过：22项Node、fmt、Clippy、321项Rust通过（2项忽略）、arm64 release构建。源码包305项全部源码覆盖且排除内部/私人材料，证据唯一编号/相对链接、git diff --check及Semifold status通过。阶段变更集macos-policy-isolation。
