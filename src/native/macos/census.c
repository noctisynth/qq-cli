/* Original SDK adapter: exact-file references, never volume or name matching. */
#include <libproc.h>
#include <sys/proc_info.h>

int qq_cli_file_processes(const char *path, int *pids, int bytes) {
    return proc_listpidspath(PROC_ALL_PIDS, 0, path, 0, pids, bytes);
}
int qq_cli_user_processes(unsigned int uid, int *pids, int bytes) {
    return proc_listpids(PROC_UID_ONLY, uid, pids, bytes);
}
