#define _POSIX_C_SOURCE 200809L

#include <errno.h>
#include <fcntl.h>
#include <time.h>
#include <unistd.h>

int main(int argc, char **argv) {
    if (argc != 3) {
        return 64;
    }

    pid_t child = fork();
    if (child < 0) {
        return 70;
    }
    if (child > 0) {
        return 0;
    }
    if (setsid() < 0) {
        _exit(71);
    }

    int ready = open(argv[1], O_WRONLY | O_CREAT | O_EXCL, 0600);
    if (ready < 0 && errno != EEXIST) {
        _exit(72);
    }
    if (ready >= 0) {
        close(ready);
    }

    const struct timespec poll = {.tv_sec = 0, .tv_nsec = 2000000L};
    while (access(argv[2], F_OK) != 0) {
        if (errno != ENOENT) {
            _exit(73);
        }
        nanosleep(&poll, NULL);
    }
    return 0;
}
