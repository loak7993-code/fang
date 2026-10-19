/* Leo runtime implementation. See leo_rt.h for the surface. */

#define _POSIX_C_SOURCE 200809L
#define _DEFAULT_SOURCE

#include "leo_rt.h"

#include <unistd.h>
#include <dirent.h>
#include <time.h>
#include <sys/socket.h>
#include <netdb.h>
#include <sys/types.h>
#include <sys/stat.h>

static void leo_sb_str_raw(leo_sb* sb, const char* data, int64_t len);

/* ------------------------------------------------------------------ */
/* Panic                                                               */
/* ------------------------------------------------------------------ */

void leo_panic(const char* msg, const char* file, int line) {
    fprintf(stderr, "leo: runtime error in %s at line %d: %s\n", file, line, msg);
    exit(101);
}

/* ------------------------------------------------------------------ */
/* Strings                                                             */
/* ------------------------------------------------------------------ */

leo_str* leo_str_new(const char* data, int64_t len) {
    leo_str* s = (leo_str*)malloc(sizeof(leo_str) + (size_t)len + 1);
    if (!s) leo_panic("out of memory", "<runtime>", 0);
    s->rc = 1;
    s->len = len;
    if (len > 0 && data) memcpy(s->bytes, data, (size_t)len);
    s->bytes[len] = '\0';
    return s;
}

leo_str* leo_str_concat(leo_str* a, leo_str* b) {
    leo_str* s = (leo_str*)malloc(sizeof(leo_str) + (size_t)(a->len + b->len) + 1);
    if (!s) leo_panic("out of memory", "<runtime>", 0);
    s->rc = 1;
    s->len = a->len + b->len;
    if (a->len > 0) memcpy(s->bytes, a->bytes, (size_t)a->len);
    if (b->len > 0) memcpy(s->bytes + a->len, b->bytes, (size_t)b->len);
    s->bytes[s->len] = '\0';
    return s;
}

bool leo_str_eq(leo_str* a, leo_str* b) {
    return a == b || (a->len == b->len && memcmp(a->bytes, b->bytes, (size_t)a->len) == 0);
}

int leo_str_cmp(leo_str* a, leo_str* b) {
    int64_t n = a->len < b->len ? a->len : b->len;
    int c = n > 0 ? memcmp(a->bytes, b->bytes, (size_t)n) : 0;
    if (c != 0) return c < 0 ? -1 : 1;
    if (a->len == b->len) return 0;
    return a->len < b->len ? -1 : 1;
}

leo_str* leo_str_slice(leo_str* s, int64_t start, int64_t end) {
    if (start < 0) start += s->len;
    if (end < 0) end += s->len;
    if (start < 0) start = 0;
    if (end > s->len) end = s->len;
    if (start >= end) return leo_str_new("", 0);
    return leo_str_new(s->bytes + start, end - start);
}

leo_str* leo_str_char_at(leo_str* s, int64_t i) {
    if (i < 0) i += s->len;
    if (i < 0 || i >= s->len) leo_panic("string index out of bounds", __FILE__, __LINE__);
    return leo_str_new(s->bytes + i, 1);
}

int64_t leo_str_find(leo_str* s, int64_t from, leo_str* needle) {
    if (from < 0) from = 0;
    if (needle->len == 0) return from <= s->len ? from : -1;
    if (needle->len > s->len) return -1;
    for (int64_t i = from; i + needle->len <= s->len; i++) {
        if (memcmp(s->bytes + i, needle->bytes, (size_t)needle->len) == 0) return i;
    }
    return -1;
}

static char leo_ascii_upper(char c) {
    return (c >= 'a' && c <= 'z') ? (char)(c - 'a' + 'A') : c;
}
static char leo_ascii_lower(char c) {
    return (c >= 'A' && c <= 'Z') ? (char)(c - 'A' + 'a') : c;
}

leo_str* leo_str_upper(leo_str* s) {
    leo_str* out = leo_str_new(s->bytes, s->len);
    for (int64_t i = 0; i < out->len; i++) out->bytes[i] = leo_ascii_upper(out->bytes[i]);
    return out;
}

leo_str* leo_str_lower(leo_str* s) {
    leo_str* out = leo_str_new(s->bytes, s->len);
    for (int64_t i = 0; i < out->len; i++) out->bytes[i] = leo_ascii_lower(out->bytes[i]);
    return out;
}

leo_str* leo_str_replace(leo_str* s, leo_str* from, leo_str* to) {
    if (from->len == 0) return leo_str_new(s->bytes, s->len);
    leo_sb* sb = leo_sb_new();
    int64_t i = 0;
    while (i < s->len) {
        int64_t at = leo_str_find(s, i, from);
        if (at < 0) {
            leo_sb_str_raw(sb, s->bytes + i, s->len - i);
            break;
        }
        leo_sb_str_raw(sb, s->bytes + i, at - i);
        leo_sb_str_raw(sb, to->bytes, to->len);
        i = at + from->len;
    }
    return leo_sb_build(sb);
}

leo_str* leo_str_strip(leo_str* s) {
    int64_t a = 0, b = s->len;
    while (a < b && (s->bytes[a] == ' ' || s->bytes[a] == '\t' || s->bytes[a] == '\n' || s->bytes[a] == '\r')) a++;
    while (b > a && (s->bytes[b - 1] == ' ' || s->bytes[b - 1] == '\t' || s->bytes[b - 1] == '\n' || s->bytes[b - 1] == '\r')) b--;
    return leo_str_new(s->bytes + a, b - a);
}

bool leo_str_starts_with(leo_str* s, leo_str* prefix) {
    if (prefix->len > s->len) return false;
    return memcmp(s->bytes, prefix->bytes, (size_t)prefix->len) == 0;
}

bool leo_str_ends_with(leo_str* s, leo_str* suffix) {
    if (suffix->len > s->len) return false;
    return memcmp(s->bytes + s->len - suffix->len, suffix->bytes, (size_t)suffix->len) == 0;
}

leo_str* leo_str_from_i64(int64_t v) {
    char buf[32];
    int n = snprintf(buf, sizeof buf, "%lld", (long long)v);
    return leo_str_new(buf, n);
}

leo_str* leo_str_from_f64(double v) {
    char buf[40];
    /* shortest representation that round-trips */
    for (int prec = 1; prec <= 17; prec++) {
        int n = snprintf(buf, sizeof buf, "%.*g", prec, v);
        if (strtod(buf, NULL) == v) return leo_str_new(buf, n);
    }
    int n = snprintf(buf, sizeof buf, "%g", v);
    return leo_str_new(buf, n);
}

leo_str* leo_str_from_bool(bool v) {
    return leo_str_new(v ? "true" : "false", v ? 4 : 5);
}

int64_t leo_str_to_i64(leo_str* s) {
    if (s->len == 0) leo_panic("to_int: the string is empty", __FILE__, __LINE__);
    char* end = NULL;
    long long v = strtoll(s->bytes, &end, 10);
    if (end != s->bytes + s->len)
        leo_panic("to_int: not a whole number", __FILE__, __LINE__);
    return (int64_t)v;
}

double leo_str_to_f64(leo_str* s) {
    if (s->len == 0) leo_panic("to_float: the string is empty", __FILE__, __LINE__);
    char* end = NULL;
    double v = strtod(s->bytes, &end);
    if (end != s->bytes + s->len)
        leo_panic("to_float: not a number", __FILE__, __LINE__);
    return v;
}

uint64_t leo_str_hash(leo_str* s) {
    /* FNV-1a */
    uint64_t h = 1469598103934665603ULL;
    for (int64_t i = 0; i < s->len; i++) {
        h ^= (uint8_t)s->bytes[i];
        h *= 1099511628211ULL;
    }
    return h;
}

/* ------------------------------------------------------------------ */
/* Print                                                               */
/* ------------------------------------------------------------------ */

void leo_print_str(leo_str* s) {
    fwrite(s->bytes, 1, (size_t)s->len, stdout);
}
void leo_print_i64(int64_t v) {
    printf("%lld", (long long)v);
}
void leo_print_f64(double v) {
    /* same shortest round-trip rule as str_from_f64 */
    char buf[40];
    for (int prec = 1; prec <= 17; prec++) {
        int n = snprintf(buf, sizeof buf, "%.*g", prec, v);
        if (strtod(buf, NULL) == v) {
            fwrite(buf, 1, (size_t)n, stdout);
            return;
        }
    }
    printf("%g", v);
}
void leo_print_bool(bool v) {
    fputs(v ? "true" : "false", stdout);
}
void leo_print_ptr(void* v) {
    printf("%p", v);
}
void leo_print_sep(void) {
    putchar(' ');
}
void leo_print_nl(void) {
    putchar('\n');
    fflush(stdout);
}

/* ------------------------------------------------------------------ */
/* raise / try / except                                                */
/* ------------------------------------------------------------------ */

void leo_raise(leo_str* value) {
    extern leo_handler* leo_handler_top;
    if (!leo_handler_top) {
        fprintf(stderr, "leo: uncaught error: ");
        fwrite(value->bytes, 1, (size_t)value->len, stderr);
        fputc('\n', stderr);
        exit(1);
    }
    leo_handler_top->value = value;
    longjmp(leo_handler_top->buf, 1);
}

leo_handler* leo_handler_top = NULL;

/* ------------------------------------------------------------------ */
/* String builder                                                      */
/* ------------------------------------------------------------------ */

leo_sb* leo_sb_new(void) {
    leo_sb* sb = (leo_sb*)malloc(sizeof(leo_sb));
    sb->cap = 32;
    sb->len = 0;
    sb->data = (char*)malloc(32);
    return sb;
}

static void leo_sb_grow(leo_sb* sb, int64_t extra) {
    if (sb->len + extra + 1 <= sb->cap) return;
    while (sb->len + extra + 1 > sb->cap) sb->cap *= 2;
    sb->data = (char*)realloc(sb->data, (size_t)sb->cap);
}

void leo_sb_str_raw(leo_sb* sb, const char* data, int64_t len) {
    leo_sb_grow(sb, len);
    memcpy(sb->data + sb->len, data, (size_t)len);
    sb->len += len;
}

void leo_sb_str(leo_sb* sb, leo_str* s) {
    leo_sb_str_raw(sb, s->bytes, s->len);
}
void leo_sb_i64(leo_sb* sb, int64_t v) {
    char buf[32];
    int n = snprintf(buf, sizeof buf, "%lld", (long long)v);
    leo_sb_str_raw(sb, buf, n);
}
void leo_sb_f64(leo_sb* sb, double v) {
    char buf[40];
    for (int prec = 1; prec <= 17; prec++) {
        int n = snprintf(buf, sizeof buf, "%.*g", prec, v);
        if (strtod(buf, NULL) == v) {
            leo_sb_str_raw(sb, buf, n);
            return;
        }
    }
    leo_sb_str_raw(sb, buf, snprintf(buf, sizeof buf, "%g", v));
}
void leo_sb_bool(leo_sb* sb, bool v) {
    leo_sb_str_raw(sb, v ? "true" : "false", v ? 4 : 5);
}

leo_str* leo_sb_build(leo_sb* sb) {
    leo_str* s = leo_str_new(sb->data, sb->len);
    free(sb->data);
    free(sb);
    return s;
}

/* ------------------------------------------------------------------ */
/* OS, filesystem, time                                                */
/* ------------------------------------------------------------------ */

static int g_argc = 0;
static char** g_argv = NULL;

void leo_os_init(int argc, char** argv) {
    g_argc = argc;
    g_argv = argv;
}

int64_t leo_os_argc(void) {
    return g_argc;
}

leo_str* leo_os_arg(int64_t i) {
    if (i < 0 || i >= g_argc) leo_panic("os.args index out of bounds", __FILE__, __LINE__);
    int64_t len = (int64_t)strlen(g_argv[i]);
    return leo_str_new(g_argv[i], len);
}

leo_str* leo_os_env(leo_str* name) {
    const char* v = getenv(name->bytes);
    if (!v) return leo_str_new("", 0);
    return leo_str_new(v, (int64_t)strlen(v));
}

leo_str* leo_os_getcwd(void) {
    char buf[4096];
    if (!getcwd(buf, sizeof buf)) leo_panic("getcwd failed", __FILE__, __LINE__);
    return leo_str_new(buf, (int64_t)strlen(buf));
}

int64_t leo_os_system(leo_str* cmd) {
    return (int64_t)system(cmd->bytes);
}

void leo_os_exit(int64_t code) {
    exit((int)code);
}

leo_str* leo_fs_read_file(leo_str* path) {
    FILE* f = fopen(path->bytes, "rb");
    if (!f) {
        fprintf(stderr, "leo: cannot open '%s' for reading\n", path->bytes);
        leo_panic("fs.read_file failed", __FILE__, __LINE__);
    }
    fseek(f, 0, SEEK_END);
    long size = ftell(f);
    fseek(f, 0, SEEK_SET);
    leo_str* s = (leo_str*)malloc(sizeof(leo_str) + (size_t)size + 1);
    if (!s) leo_panic("out of memory", "<runtime>", 0);
    s->rc = 1;
    s->len = size;
    if (size > 0 && fread(s->bytes, 1, (size_t)size, f) != (size_t)size) {
        fclose(f);
        leo_panic("fs.read_file failed mid-read", __FILE__, __LINE__);
    }
    s->bytes[size] = '\0';
    fclose(f);
    return s;
}

void leo_fs_write_file(leo_str* path, leo_str* data) {
    FILE* f = fopen(path->bytes, "wb");
    if (!f) {
        fprintf(stderr, "leo: cannot open '%s' for writing\n", path->bytes);
        leo_panic("fs.write_file failed", __FILE__, __LINE__);
    }
    if (data->len > 0) fwrite(data->bytes, 1, (size_t)data->len, f);
    fclose(f);
}

void leo_fs_append_file(leo_str* path, leo_str* data) {
    FILE* f = fopen(path->bytes, "ab");
    if (!f) {
        fprintf(stderr, "leo: cannot open '%s' for appending\n", path->bytes);
        leo_panic("fs.append_file failed", __FILE__, __LINE__);
    }
    if (data->len > 0) fwrite(data->bytes, 1, (size_t)data->len, f);
    fclose(f);
}

bool leo_fs_exists(leo_str* path) {
    return access(path->bytes, F_OK) == 0;
}

leo_str* leo_fs_list_dir(leo_str* path) {
    /* Newline-joined entry names; the std module splits it. */
    leo_sb* sb = leo_sb_new();
    DIR* d = opendir(path->bytes);
    if (!d) {
        fprintf(stderr, "leo: cannot list directory '%s'\n", path->bytes);
        leo_panic("fs.list_dir failed", __FILE__, __LINE__);
    }
    struct dirent* e;
    bool first = true;
    while ((e = readdir(d)) != NULL) {
        if (strcmp(e->d_name, ".") == 0 || strcmp(e->d_name, "..") == 0) continue;
        if (!first) leo_sb_str_raw(sb, "\n", 1);
        first = false;
        leo_sb_str_raw(sb, e->d_name, (int64_t)strlen(e->d_name));
    }
    closedir(d);
    return leo_sb_build(sb);
}

void leo_fs_mkdir(leo_str* path) {
    if (mkdir(path->bytes, 0755) != 0) {
        fprintf(stderr, "leo: cannot create directory '%s'\n", path->bytes);
        leo_panic("fs.mkdir failed", __FILE__, __LINE__);
    }
}

void leo_fs_remove(leo_str* path) {
    if (remove(path->bytes) != 0) {
        fprintf(stderr, "leo: cannot remove '%s'\n", path->bytes);
        leo_panic("fs.remove failed", __FILE__, __LINE__);
    }
}

double leo_time_now(void) {
    struct timespec ts;
    clock_gettime(CLOCK_REALTIME, &ts);
    return (double)ts.tv_sec + (double)ts.tv_nsec * 1e-9;
}

double leo_time_clock(void) {
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return (double)ts.tv_sec + (double)ts.tv_nsec * 1e-9;
}

void leo_time_sleep(double seconds) {
    struct timespec ts;
    ts.tv_sec = (time_t)seconds;
    ts.tv_nsec = (long)((seconds - (double)ts.tv_sec) * 1e9);
    nanosleep(&ts, NULL);
}

leo_str* leo_input_line(void) {
    char* line = NULL;
    size_t cap = 0;
    ssize_t n = getline(&line, &cap, stdin);
    if (n < 0) {
        free(line);
        return leo_str_new("", 0);
    }
    while (n > 0 && (line[n - 1] == '\n' || line[n - 1] == '\r')) n--;
    leo_str* s = leo_str_new(line, n);
    free(line);
    return s;
}

/* ------------------------------------------------------------------ */
/* Networking                                                          */
/* ------------------------------------------------------------------ */

void* leo_net_resolve(leo_str* host, int64_t port) {
    char portbuf[16];
    snprintf(portbuf, sizeof portbuf, "%lld", (long long)port);
    struct addrinfo hints, *res = NULL;
    memset(&hints, 0, sizeof hints);
    hints.ai_family = AF_INET;
    hints.ai_socktype = SOCK_STREAM;
    int rc = getaddrinfo(host->bytes, portbuf, &hints, &res);
    if (rc != 0 || !res) {
        fprintf(stderr, "leo: cannot resolve '%s'\n", host->bytes);
        leo_panic("net resolve failed", __FILE__, __LINE__);
    }
    int fd = socket(res->ai_family, res->ai_socktype, res->ai_protocol);
    if (fd < 0) {
        freeaddrinfo(res);
        leo_panic("socket creation failed", __FILE__, __LINE__);
    }
    if (connect(fd, res->ai_addr, res->ai_addrlen) != 0) {
        close(fd);
        freeaddrinfo(res);
        fprintf(stderr, "leo: cannot connect to '%s' port %lld\n", host->bytes, (long long)port);
        leo_panic("net connect failed", __FILE__, __LINE__);
    }
    freeaddrinfo(res);
    return (void*)(intptr_t)fd;
}

int64_t leo_tcp_send(void* sock, leo_str* data) {
    int fd = (int)(intptr_t)sock;
    int64_t total = 0;
    while (total < data->len) {
        ssize_t n = send(fd, data->bytes + total, (size_t)(data->len - total), 0);
        if (n <= 0) leo_panic("tcp send failed", __FILE__, __LINE__);
        total += n;
    }
    return total;
}

leo_str* leo_tcp_recv_all(void* sock) {
    int fd = (int)(intptr_t)sock;
    leo_sb* sb = leo_sb_new();
    char buf[16384];
    for (;;) {
        ssize_t n = recv(fd, buf, sizeof buf, 0);
        if (n < 0) leo_panic("tcp receive failed", __FILE__, __LINE__);
        if (n == 0) break;
        leo_sb_str_raw(sb, buf, n);
    }
    return leo_sb_build(sb);
}

void leo_tcp_close(void* sock) {
    close((int)(intptr_t)sock);
}

/* ------------------------------------------------------------------ */
/* Sorting                                                             */
/* ------------------------------------------------------------------ */

static int leo_cmp_i64(const void* a, const void* b) {
    int64_t x = *(const int64_t*)a, y = *(const int64_t*)b;
    return x < y ? -1 : x > y ? 1 : 0;
}
static int leo_cmp_f64(const void* a, const void* b) {
    double x = *(const double*)a, y = *(const double*)b;
    return x < y ? -1 : x > y ? 1 : 0;
}
static int leo_cmp_str(const void* a, const void* b) {
    leo_str* x = *(leo_str* const*)a;
    leo_str* y = *(leo_str* const*)b;
    return leo_str_cmp(x, y);
}

void leo_sort_i64(int64_t* base, int64_t n) {
    qsort(base, (size_t)n, sizeof(int64_t), leo_cmp_i64);
}
void leo_sort_f64(double* base, int64_t n) {
    qsort(base, (size_t)n, sizeof(double), leo_cmp_f64);
}
void leo_sort_str(leo_str** base, int64_t n) {
    qsort(base, (size_t)n, sizeof(leo_str*), leo_cmp_str);
}
