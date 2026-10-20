# 真实监听原生取消

文档编号：CHECK-044。状态：正常原生Ctrl+C功能通过。更新：2026-10-20。
关联：SPEC-004、TASK-002、CHECK-030、CHECK-043。基线cfb8816，用户明确授权继续Ctrl+C验收；仅原实验会话有界只读监听，不发送或获取历史。

以正常release公开CLI执行event watch --conversation CONVERSATION --timeout 120。辅助Python在独立CREATE_NEW_CONSOLE、STARTF_USESHOWWINDOW隐藏控制台启动CLI，等待8秒后发送真正GenerateConsoleCtrlEvent(CTRL_C_EVENT,0)；不是kill/取消标志模拟，不向用户当前控制台广播。辅助进程在子CLI启动后才忽略自身控制信号，避免子进程继承忽略。私人目标与脚本/输出均忽略。

CLI先输出ready，控制信号前仍存活；信号成功，0.297秒内exitCode130，stderr0字节。NDJSON恰为ready及summary，reason=cancelled、emitted0、duplicates0、gaps0、frames1；无新业务事件或正文输出。随后status原账号v4.18.38 ready/online，证明监听结束后实例仍可用，没有调用napcat stop/remove。

进程快照均17个，16个PID及创建时间相同，1个旧进程替换为新进程；原因未确认，不宣称所有QQ进程未变，也不推断由Ctrl+C导致。首次同步脚本因要求17个全同的断言失败，未写文档/未提交；如实保留观察后同步。不为此扩展稳定性专项。

无产品代码修复，不新增镜像实现测试；文档同步后diff/隐私/链接及Semifold核对。既有事件正常截止及双向消息证据保留，本项不提升其它notice、断线补读、阻塞输出及稳定性专项。TASK-002原混合计数67/100保持，正常取消子项单列。下一步原实例停止/注销，安装及旧记录保留。
