#include <vector>
#include <cstdio>
#include <cstdint>
#include <algorithm>
#include <random>
int main() {
    std::mt19937_64 rng(42);
    std::vector<int64_t> xs;
    xs.reserve(1000000);
    for (int i = 0; i < 1000000; i++)
        xs.push_back((int64_t)(rng() % 1000000000));
    std::sort(xs.begin(), xs.end());
    printf("%lld %lld %lld\n", (long long)xs[0], (long long)xs[500000], (long long)xs[999999]);
    return 0;
}
