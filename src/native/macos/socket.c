/* Original, bounded SDK adapter. No enumeration, socket I/O, or retained pointers. */
#include <libproc.h>
#include <sys/proc_info.h>
#include <sys/socket.h>
#include <netinet/in.h>
#include <stdint.h>
#include <stddef.h>
#include <string.h>

struct qq_cli_tcp_snapshot {
    uint64_t socket_id;
    uint64_t generation;
    uint32_t family;
    uint16_t local_port;
    uint16_t remote_port;
    uint8_t local_addr[16];
    uint8_t remote_addr[16];
};

_Static_assert(sizeof(struct qq_cli_tcp_snapshot) == 56, "Unexpected snapshot ABI size");
_Static_assert(offsetof(struct qq_cli_tcp_snapshot, local_addr) == 24, "Unexpected address offset");

/* -1 means unverifiable; 0 is another socket kind/state; 1 is an established TCP snapshot. */
int qq_cli_tcp_snapshot(int pid, int fd, struct qq_cli_tcp_snapshot *out) {
    struct socket_fdinfo info;
    memset(out, 0, sizeof(*out));
    memset(&info, 0, sizeof(info));
    int count = proc_pidfdinfo(pid, fd, PROC_PIDFDSOCKETINFO, &info, sizeof(info));
    if (count != sizeof(info)) return -1;
    const struct socket_info *socket = &info.psi;
    if (socket->soi_kind != SOCKINFO_TCP || socket->soi_type != SOCK_STREAM ||
        socket->soi_protocol != IPPROTO_TCP) return 0;
    const struct tcp_sockinfo *tcp = &socket->soi_proto.pri_tcp;
    if (tcp->tcpsi_state != TSI_S_ESTABLISHED) return 0;
    const struct in_sockinfo *ip = &tcp->tcpsi_ini;
    if (socket->soi_family == AF_INET && ip->insi_vflag == INI_IPV4) {
        out->family = 4;
        memcpy(out->local_addr, &ip->insi_laddr.ina_46.i46a_addr4, 4);
        memcpy(out->remote_addr, &ip->insi_faddr.ina_46.i46a_addr4, 4);
    } else if (socket->soi_family == AF_INET6 && ip->insi_vflag == INI_IPV6 &&
               ip->insi_v6.in6_ifindex == 0 && ip->insi_flow == 0) {
        out->family = 6;
        memcpy(out->local_addr, &ip->insi_laddr.ina_6, 16);
        memcpy(out->remote_addr, &ip->insi_faddr.ina_6, 16);
    } else return 0;
    if (ip->insi_lport <= 0 || ip->insi_lport > UINT16_MAX ||
        ip->insi_fport <= 0 || ip->insi_fport > UINT16_MAX ||
        socket->soi_so == 0 || ip->insi_gencnt == 0) return -1;
    out->socket_id = socket->soi_so;
    out->generation = ip->insi_gencnt;
    out->local_port = ntohs((uint16_t)ip->insi_lport);
    out->remote_port = ntohs((uint16_t)ip->insi_fport);
    return 1;
}
