#include <cstdio>
#include <cstdint>

static int64_t fib(int64_t n) {
    if (n < 2) return n;
    return fib(n - 1) + fib(n - 2);
}

int main() {
    int64_t total = 0;
    for (int i = 0; i < 32; i++) total += fib(i);
    printf("%lld\n", (long long)total);
    return 0;
}
