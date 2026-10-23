/* Original audit-token adapter. Only explicit target PIDs; no enumeration/debug access. */
#include <mach/mach.h>
#include <libproc.h>
#include <signal.h>
#include <stdint.h>
#include <string.h>

_Static_assert(sizeof(audit_token_t) == 8 * sizeof(uint32_t), "Unexpected audit token size");

int qq_cli_capture_audit_token(int pid, uint32_t output[8]) {
    mach_port_t name = MACH_PORT_NULL;
    kern_return_t result = task_name_for_pid(mach_task_self(), pid, &name);
    if (result != KERN_SUCCESS) return -1;
    audit_token_t token = {{0}};
    mach_msg_type_number_t count = TASK_AUDIT_TOKEN_COUNT;
    result = task_info(name, TASK_AUDIT_TOKEN, (task_info_t)&token, &count);
    kern_return_t released = mach_port_deallocate(mach_task_self(), name);
    if (result != KERN_SUCCESS || released != KERN_SUCCESS || count != TASK_AUDIT_TOKEN_COUNT) return -1;
    memcpy(output, &token, sizeof(token));
    return 0;
}

int qq_cli_terminate_audit_token(const uint32_t input[8]) {
    audit_token_t token;
    memcpy(&token, input, sizeof(token));
    /* Kernel validates the original process generation; no fallback to numeric PID signalling. */
    return proc_signal_with_audittoken(&token, SIGKILL);
}

int qq_cli_suspend_audit_token(const uint32_t input[8]) {
    audit_token_t token;
    memcpy(&token, input, sizeof(token));
    return proc_signal_with_audittoken(&token, SIGSTOP);
}

int qq_cli_resume_audit_token(const uint32_t input[8]) {
    audit_token_t token;
    memcpy(&token, input, sizeof(token));
    return proc_signal_with_audittoken(&token, SIGCONT);
}
