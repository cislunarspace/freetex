/* glibc 2.38+ 头文件会把 strtoll 等重定向为 __isoc23_strtoll；pyke 预编译的
 * onnxruntime 静态库引用这些符号，Ubuntu 22.04（glibc 2.35）没有。此处提供
 * 转发到经典实现的 shim，使同一二进制可运行在 22.04/24.04/26.04 上。
 *
 * glibc 2.38+ headers redirect strtoll etc. to __isoc23_strtoll; pyke's
 * prebuilt onnxruntime static archives reference them, and Ubuntu 22.04
 * (glibc 2.35) lacks them. This shim forwards to the classic functions so one
 * binary runs on 22.04/24.04/26.04 alike. */

#include <stdlib.h>
#include <inttypes.h>

/* 防止在 glibc ≥2.38 上编译本文件时，经典名被宏重定向成 __isoc23_* 造成自递归 */
/* prevent the classic names from being macro-redirected into __isoc23_* (which
 * would make these definitions recurse) when built against glibc ≥2.38 */
#undef strtol
#undef strtoul
#undef strtoll
#undef strtoull
#undef strtoimax
#undef strtoumax

long __isoc23_strtol(const char *nptr, char **endptr, int base) {
    return strtol(nptr, endptr, base);
}

unsigned long __isoc23_strtoul(const char *nptr, char **endptr, int base) {
    return strtoul(nptr, endptr, base);
}

long long __isoc23_strtoll(const char *nptr, char **endptr, int base) {
    return strtoll(nptr, endptr, base);
}

unsigned long long __isoc23_strtoull(const char *nptr, char **endptr, int base) {
    return strtoull(nptr, endptr, base);
}

intmax_t __isoc23_strtoimax(const char *nptr, char **endptr, int base) {
    return strtoimax(nptr, endptr, base);
}

uintmax_t __isoc23_strtoumax(const char *nptr, char **endptr, int base) {
    return strtoumax(nptr, endptr, base);
}
