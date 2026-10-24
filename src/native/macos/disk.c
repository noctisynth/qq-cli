/* Original SDK adapter: one ioctl on a held character device, never a path. */
#include <sys/types.h>
#include <sys/stat.h>
#include <sys/disk.h>
#include <sys/ioctl.h>
#include <unistd.h>
#include <stdint.h>
#include <errno.h>

int qq_cli_disk_matches(int fd, uint32_t expected_major, uint32_t expected_minor) {
    struct stat value;
    if (fstat(fd, &value) != 0 || !S_ISCHR(value.st_mode) || value.st_uid != geteuid()) return -1;
    return ((uint32_t)major(value.st_rdev) == expected_major &&
            (uint32_t)minor(value.st_rdev) == expected_minor) ? 0 : -1;
}
int qq_cli_disk_eject_once(int fd, uint32_t expected_major, uint32_t expected_minor) {
    if (qq_cli_disk_matches(fd, expected_major, expected_minor) != 0) return EINVAL;
    if (ioctl(fd, DKIOCEJECT, NULL) == 0) return 0;
    return errno;
}
