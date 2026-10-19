#include <cstdio>
#include <cstdint>
int main() {
    int64_t total = 0;
    for (int64_t i = 0; i < 100000000; i++) total += i;
    printf("%lld\n", (long long)total);
    return 0;
}
