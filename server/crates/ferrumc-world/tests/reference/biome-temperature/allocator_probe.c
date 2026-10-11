/* External test-only Linux/glibc measurement; never linked into production.
 * The Rust probe emits write boundaries. Counters are thread local, and all
 * reports bypass these hooks via syscall. The positive control must allocate.
 */
#define _GNU_SOURCE
#include <stddef.h>
#include <stdint.h>
#include <string.h>
#include <stdio.h>
#include <sys/syscall.h>
#include <unistd.h>
#include <errno.h>
#include <pthread.h>

extern void *__libc_malloc(size_t);
extern void *__libc_calloc(size_t, size_t);
extern void *__libc_realloc(void *, size_t);
extern void *__libc_memalign(size_t, size_t);
extern void __libc_free(void *);

static _Thread_local int active;
static _Thread_local size_t calls, requested, frees;
static _Thread_local char phase[32];
static _Thread_local unsigned char *stack_low, *stack_edge;
static _Thread_local uintptr_t stack_reference;

/* External high-water observation of stack writes below a 16 KiB protected
 * marker-frame margin. Never write live frames or a guard page. This is separate
 * from the exact native object-capacity accounting in Rust's parser test.
 */
static void mark_stack(void) {
    pthread_attr_t attr;
    void *base = NULL;
    size_t size = 0;
    if (pthread_getattr_np(pthread_self(), &attr) || pthread_attr_getstack(&attr, &base, &size)) _exit(88);
    pthread_attr_destroy(&attr);
    stack_reference = (uintptr_t)__builtin_frame_address(0);
    stack_low = (unsigned char *)base + 4096;
    stack_edge = (unsigned char *)(stack_reference - 16384);
    if ((uintptr_t)stack_edge < (uintptr_t)stack_low || (uintptr_t)stack_edge >= (uintptr_t)base + size) _exit(89);
    memset(stack_low, 0xa7, (size_t)(stack_edge - stack_low));
}
static size_t stack_writes(void) {
    for (unsigned char *p = stack_low; p < stack_edge; ++p) {
        if (*p != 0xa7) return stack_reference - (uintptr_t)p;
    }
    return 16384;
}

static void charge(size_t bytes) {
    if (active) { ++calls; requested += bytes; }
}
void *malloc(size_t size) { charge(size); return __libc_malloc(size); }
void *calloc(size_t count, size_t size) { charge(count * size); return __libc_calloc(count, size); }
void *realloc(void *p, size_t size) { charge(size); return __libc_realloc(p, size); }
void *aligned_alloc(size_t alignment, size_t size) { charge(size); return __libc_memalign(alignment, size); }
void *memalign(size_t alignment, size_t size) { charge(size); return __libc_memalign(alignment, size); }
int posix_memalign(void **p, size_t alignment, size_t size) {
    if (alignment < sizeof(void *) || alignment % sizeof(void *) || (alignment & (alignment - 1))) return EINVAL;
    charge(size);
    void *result = __libc_memalign(alignment, size);
    if (!result) return ENOMEM;
    *p = result;
    return 0;
}
void free(void *p) { if (active && p) ++frees; __libc_free(p); }

ssize_t write(int fd, const void *buffer, size_t count) {
    const char *text = buffer;
    if (fd == 1 && count > 13 && !memcmp(text, "GEN057_END_", 11)) {
        active = 0;
        char report[256];
        int bytes = snprintf(report, sizeof(report), "GEN057_ALLOC phase=%s calls=%zu bytes=%zu frees=%zu stack_write_bytes=%zu\n", phase, calls, requested, frees, stack_writes());
        syscall(SYS_write, 2, report, (size_t)bytes);
    }
    ssize_t result = syscall(SYS_write, fd, buffer, count);
    if (fd == 1 && count > 14 && !memcmp(text, "GEN057_BEGIN_", 13)) {
        size_t length = count - 14;
        if (length >= sizeof(phase)) length = sizeof(phase) - 1;
        memcpy(phase, text + 13, length); phase[length] = '\0';
        calls = requested = frees = 0;
        mark_stack();
        active = 1;
    }
    return result;
}
