# macOS 重签名载荷保全

文档编号：CHECK-103。日期：2026-10-24。状态：合成测试、实际安装只读探测及macOS门禁通过。
关联：SPEC-008、TASK-006。起点：89384e4。

签名计划保存独立v1载荷摘要，共享thin/fat32/fat64解析，覆盖全部既有slice；只规范化LC_CODE_SIGNATURE.datasize与__LINKEDIT的filesize/vmsize，排除有界末尾签名分配区。其他命令、代码、数据、CPU/subtype、容器格式及slice顺序/alignment保持。fat物理偏移允许重排，但所有非slice填充必须为零。重复架构、越界/重叠slice、非零fat64保留字段拒绝。LINKEDIT必须无section、只读、位于其他segment之后且终止于slice末尾；签名头与声明长度、文件/命令预算均受限。

原创结构夹具不具有有效系统签名，仅验证签名分配增长、fat64重排、非零填充拒绝、不可变字节变化、错误命令/权限/长度/VM大小/保留字段、截断及截止。原创真实签名夹具包括Rust测试程序、C动态库和arm64/x86_64通用C程序，不执行重签后的副本。嵌套目标依序按角色重签后摘要相同；修改主程序代码后重新生成有效系统签名仍拒绝。通用程序更换权益后摘要相同，修改x86_64的__text后重新生成有效签名仍拒绝，证明外来slice未遗漏；不宣称Intel运行支持。

测试发现系统codesign可留下SuperBlob声明长度之后的非零分配填充。初版要求该填充全零导致原创Rust夹具失败；最终将完整、严格有界的LC_CODE_SIGNATURE分配作为可变签名区域，保留独立OS有效性验证。不能把此例外扩展到fat填充或非签名代码/数据。

SigningPlan.verify_payloads只比较原目标载荷并前后重核新Inventory。不是允许任意资源/属性变化的整树验收，不检测额外目标，不验证新权益，不授予写权限或部署凭据；后续受控签名事务仍需独立完成这些检查。正常revalidate仍拒绝签名后新指纹。未新增unsafe、依赖或生产codesign调用。

最终macOS门禁通过：22项Node、fmt、Clippy、315项Rust通过（2项忽略）、arm64 release构建。Semifold阶段变更集macos-signing-payload；源码包302项，全部源码覆盖、内部/私人材料排除、证据编号及相对链接、git diff --check通过。Windows原生/PowerShell门禁未执行。不复制、重签名、启动真实QQ；无真实账号操作。仅本地提交，不推送或发布。

最终代码对已安装QQ 7.0.2-53644/build 53644只读探测通过：3578节点、47个显式bundle元数据项、89个签名目标。89个通用映像的178个既有slice均通过载荷解析与摘要建立，计划创建后的原树复核一致。嵌套角色仍使用Native占位，仅验证实际布局和载荷覆盖，不代表生产角色审查。未对真实QQ执行重签名后比较。
