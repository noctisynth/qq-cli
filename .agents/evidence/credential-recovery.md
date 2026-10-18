# 原生凭据恢复与并发核对

文档编号：CHECK-021。状态：跨进程协调有界离线验收，底层根因未定。更新：2026-10-19。
关联：CHECK-020、SPEC-004、RFC-0003、TASK-002。

CHECK-020已记录原签名密钥删除/恢复后父进程或独立CLI得到CREDENTIAL_NOT_FOUND，完整回归未通过。最小原创程序脱离QQ/Job，在4个临时命名空间各20轮删除/恢复、独立进程读取缺失时观察到读取不一致。没有读取真实账号凭据。

同一诊断仅在测试层包装进程内互斥后连续3轮通过；恢复未包装版本的一次对照和后续5轮也通过。随后在生产适配器实验进程内串行候选：首轮6项通过，压力重复4轮通过后，第5轮原歧义/密钥恢复用例仍报CREDENTIAL_NOT_FOUND；候选无效，生产修改撤回。原并发测试不被串行化；不以对照通过认定底层根因，不新增重试/睡眠补偿、持久化变更或自动造密钥。

固定依赖keyring 3.6.3的windows.rs由Cargo.lock定位，源码注释提醒同条目跨线程操作顺序不保证；不同命名空间现象仍不能由此直接解释。完整跨进程/故障验收仍独立保留。

跨进程对照：仅在独立诊断的每次get/set/delete外持同一临时空文件锁，4线程/80个原创缺失读取子进程保持，连续20轮通过（1600轮恢复）；不作为生产mutex有效性的替代证明。下一候选为native独立凭据锁边界：会话内固定命名对象，仅SYNCHRONIZE|MUTEX_MODIFY_STATE、非继承、有限等待和线程绑定RAII；不修改凭据内容/命名/持久化，不锁整套测试。只有原并发回归通过才考虑保留。

2026-10-19核对Microsoft可变文档：[CreateMutexW](https://learn.microsoft.com/en-us/windows/win32/api/synchapi/nf-synchapi-createmutexw)、[Using Mutex Objects](https://learn.microsoft.com/en-us/windows/win32/sync/using-mutex-objects)、[ReleaseMutex](https://learn.microsoft.com/en-us/windows/win32/api/synchapi/nf-synchapi-releasemutex)。只确认命名跨进程共享、默认令牌DACL/不继承、同线程释放、有限等待及abandoned资源不应直接继续；不以文档推定凭据故障根因。

## 保留的实现与验收

采用独立native::credential_lock边界，CreateMutexExW请求SYNCHRONIZE|MUTEX_MODIFY_STATE；默认当前令牌DACL、非继承、无初始拥有者、当前Windows会话内固定qq-cli命名对象。每次get/set/delete持一份线程绑定Guard，最多等待1000ms；超时/不可访问返回CREDENTIAL_UNAVAILABLE，不调用凭据存储。abandoned虽获得所有权，仍先释放并返回错误，不尝试修复存储。Guard以Rc标记禁止Send/Sync，所有权在原线程释放、唯一句柄关闭；无凭据值或业务状态进入native模块。条目身份和持久化不变，状态/生命周期锁不被代替。

2项独立原生测试通过：唯一原创对象验证非继承、其他线程超时及析构后可获取；原创子进程不执行析构而退出，持另一个只持有对象的句柄保持名字，确认abandoned拒绝后不同线程可取得，证明失败路径释放原线程所有权。最小权利和无初始拥有者另核对[CreateMutexExW](https://learn.microsoft.com/en-us/windows/win32/api/synchapi/nf-synchapi-createmutexexw)（2026-10-19，可变来源）。

原始并发测试不再用测试层包装，4线程及80个原创缺失读取子进程仍在。生产协调实现的原生恢复1项和原stop 5项，连续10轮通过（60次测试执行，800轮独立凭据恢复）；原先两条恢复用例本轮未再次失败。最终完整253项离线回归通过，clippy/fmt/release、架构和190文件源码包通过；Semifold47条changeset未消费。真实QQ未操作。

本结论是有界协调缓解有效，不宣称确定Windows/keyring内部根因、证明所有跨用户/会话/外部凭据管理客户端或崩溃矩阵。保留原失败历史；完整凭据全路径及生命周期任务仍待完成，业务62/95不变。
