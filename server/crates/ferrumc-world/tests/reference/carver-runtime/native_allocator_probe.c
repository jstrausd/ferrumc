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
/* Bounded observer bookkeeping is external to every production category. */
#define NATIVE_SLOTS 32768
static _Thread_local uintptr_t native_keys[NATIVE_SLOTS];
static _Thread_local size_t native_sizes[NATIVE_SLOTS];
static _Thread_local size_t native_live, native_objects, native_peak, native_peak_objects;
static size_t native_index(void *p) { return ((uintptr_t)p >> 4) & (NATIVE_SLOTS - 1); }
static void native_remove(void *p) {
    if (!active || !p) return;
    size_t start = native_index(p);
    for (size_t n = 0; n < NATIVE_SLOTS; ++n) {
        size_t i = (start + n) & (NATIVE_SLOTS - 1);
        if (!native_keys[i]) return;
        if (native_keys[i] == (uintptr_t)p) {
            native_live -= native_sizes[i]; --native_objects;
            native_keys[i] = 1; native_sizes[i] = 0; return;
        }
    }
}
static void *native_add(void *p, size_t size) {
    if (!active || !p) return p;
    size_t start = native_index(p);
    for (size_t n = 0; n < NATIVE_SLOTS; ++n) {
        size_t i = (start + n) & (NATIVE_SLOTS - 1);
        if (native_keys[i] <= 1) {
            native_keys[i] = (uintptr_t)p; native_sizes[i] = size;
            native_live += size; ++native_objects;
            if (native_live > native_peak) native_peak = native_live;
            if (native_objects > native_peak_objects) native_peak_objects = native_objects;
            return p;
        }
    }
    _exit(90); /* Explicitly bounded; a missing census cannot be accepted. */
}
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
void *malloc(size_t size) { charge(size); return native_add(__libc_malloc(size), size); }
void *calloc(size_t count, size_t size) { charge(count * size); return native_add(__libc_calloc(count, size), count * size); }
void *realloc(void *p, size_t size) { charge(size); void *next = __libc_realloc(p, size); if (next || !size) native_remove(p); return native_add(next, size); }
void *aligned_alloc(size_t alignment, size_t size) { charge(size); return native_add(__libc_memalign(alignment, size), size); }
void *memalign(size_t alignment, size_t size) { charge(size); return native_add(__libc_memalign(alignment, size), size); }
int posix_memalign(void **p, size_t alignment, size_t size) {
    if (alignment < sizeof(void *) || alignment % sizeof(void *) || (alignment & (alignment - 1))) return EINVAL;
    charge(size);
    void *result = __libc_memalign(alignment, size);
    if (!result) return ENOMEM;
    *p = native_add(result, size);
    return 0;
}
void free(void *p) { if (active && p) ++frees; native_remove(p); __libc_free(p); }

ssize_t write(int fd, const void *buffer, size_t count) {
    const char *text = buffer;
    if (fd == 1 && count > 13 && !memcmp(text, "GEN057_END_", 11)) {
        active = 0;
        char report[512];
        int bytes = snprintf(report, sizeof(report), "GEN057_ALLOC phase=%s calls=%zu bytes=%zu frees=%zu stack_write_bytes=%zu native_live_bytes=%zu native_live_objects=%zu native_peak_bytes=%zu native_peak_objects=%zu\n", phase, calls, requested, frees, stack_writes(), native_live, native_objects, native_peak, native_peak_objects);
        syscall(SYS_write, 2, report, (size_t)bytes);
    }
    ssize_t result = syscall(SYS_write, fd, buffer, count);
    if (fd == 1 && count > 14 && !memcmp(text, "GEN057_BEGIN_", 13)) {
        size_t length = count - 14;
        if (length >= sizeof(phase)) length = sizeof(phase) - 1;
        memcpy(phase, text + 13, length); phase[length] = '\0';
        calls = requested = frees = 0;
        memset(native_keys, 0, sizeof(native_keys)); memset(native_sizes, 0, sizeof(native_sizes));
        native_live = native_objects = native_peak = native_peak_objects = 0;
        mark_stack();
        active = 1;
    }
    return result;
}
