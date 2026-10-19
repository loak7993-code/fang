#include <cstdio>
#include <cstdint>
#include <unordered_map>
int main() {
    std::unordered_map<int64_t, int64_t> d;
    d.reserve(100000);
    for (int64_t i = 0; i < 1000000; i++) d[i % 100000] = i;
    int64_t total = 0;
    for (int64_t i = 0; i < 1000000; i++) {
        auto it = d.find(i % 100000);
        if (it != d.end()) total += it->second;
    }
    printf("%lld\n", (long long)total);
    return 0;
}
