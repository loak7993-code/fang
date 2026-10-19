#include <string>
#include <cstdio>
int main() {
    std::string s;
    for (int i = 0; i < 100000; i++) s = s + "x";
    printf("%zu\n", s.size());
    return 0;
}
