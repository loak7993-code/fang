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
    s->cap = len;
    if (len > 0 && data) memcpy(s->bytes, data, (size_t)len);
    s->bytes[len] = '\0';
    return s;
}

leo_str* leo_str_concat(leo_str* a, leo_str* b) {
    int64_t n = a->len + b->len;
    leo_str* s = (leo_str*)malloc(sizeof(leo_str) + (size_t)n + 1);
    if (!s) leo_panic("out of memory", "<runtime>", 0);
    s->rc = 1;
    s->len = n;
    s->cap = n;
    if (a->len > 0) memcpy(s->bytes, a->bytes, (size_t)a->len);
    if (b->len > 0) memcpy(s->bytes + a->len, b->bytes, (size_t)b->len);
    s->bytes[n] = '\0';
    return s;
}

leo_str* leo_str_concat_assign(leo_str* a, leo_str* b) {
    if (a && b && a != b && a->rc == 1 && a->cap >= a->len + b->len) {
        if (b->len > 0) memcpy(a->bytes + a->len, b->bytes, (size_t)b->len);
        a->len += b->len;
        a->bytes[a->len] = '\0';
        return a;
    }
    if (!a) return leo_str_new(b ? b->bytes : "", b ? b->len : 0);
    if (!b) return a;
    int64_t n = a->len + b->len;
    int64_t cap = n * 2;
    leo_str* s = (leo_str*)malloc(sizeof(leo_str) + (size_t)cap + 1);
    if (!s) leo_panic("out of memory", "<runtime>", 0);
    s->rc = 1;
    s->len = n;
    s->cap = cap;
    if (a->len > 0) memcpy(s->bytes, a->bytes, (size_t)a->len);
    if (b->len > 0) memcpy(s->bytes + a->len, b->bytes, (size_t)b->len);
    s->bytes[n] = '\0';
    leo_str_decref(a);
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
leo_str* leo_debug_str(void* v, int kind);

static void leo_debug_sb(leo_sb* sb, void* item, int item_kind);

/* item_kind: 0=str(ptr), 1=int64, 2=f64, 3=bool, 4=debug(list) */
static void leo_debug_sb(leo_sb* sb, void* item, int item_kind) {
    switch (item_kind) {
    case 0: {
        leo_str* s = (leo_str*)item;
        leo_sb_str_raw(sb, s ? s->bytes : "", s ? s->len : 0);
        break;
    }
    case 1: leo_sb_i64(sb, *(int64_t*)item); break;
    case 2: leo_sb_f64(sb, *(double*)item); break;
    case 3: leo_sb_bool(sb, *(bool*)item); break;
    default: break;
    }
}

leo_str* leo_debug_str(void* v, int kind) {
    leo_sb* sb = leo_sb_new();
    if (kind == 6) {
        /* dict[str, int]-style: printed as {k: v}; only the common
           stampings are rendered; anything else shows its pointer. */
        leo_sb_str_raw(sb, "{...}", 5);
        return leo_sb_build(sb);
    }
    /* All Leo lists share the {rc, len, cap, elems} header. */
    struct leo_dbg_list {
        int64_t rc, len, cap;
        void* elems;
    }* l = (struct leo_dbg_list*)v;
    int64_t n = l ? l->len : 0;
    int width = (kind == 5) ? 8 : 8; /* int64/f64/ptr slots are 8 bytes */
    const char* open = (kind == 5) ? "(" : "[";
    const char* close = (kind == 5) ? ")" : "]";
    leo_sb_str_raw(sb, open, 1);
    for (int64_t i = 0; i < n; i++) {
        if (i > 0) leo_sb_str_raw(sb, ", ", 2);
        char* slot = (char*)l->elems + i * width;
        int item_kind;
        if (kind == 0) item_kind = 0;          /* list[str] */
        else if (kind == 1) item_kind = 1;     /* list[int] */
        else if (kind == 2) item_kind = 2;     /* list[float] */
        else if (kind == 4) item_kind = 3;     /* list[bool] */
        else item_kind = 0;                     /* ptr-ish: best effort */
        if (kind == 0) {
            leo_str* s = *(leo_str**)slot;
            leo_sb_str_raw(sb, "'", 1);
            leo_sb_str_raw(sb, s ? s->bytes : "", s ? s->len : 0);
            leo_sb_str_raw(sb, "'", 1);
        } else {
            leo_debug_sb(sb, slot, item_kind);
        }
    }
    leo_sb_str_raw(sb, close, 1);
    return leo_sb_build(sb);
}

void leo_print_debug(void* v, int kind) {
    leo_str* s = leo_debug_str(v, kind);
    fwrite(s->bytes, 1, (size_t)s->len, stdout);
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
    s->cap = size;
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

leo_str* leo_input_prompt(leo_str* prompt) {
    if (prompt && prompt->len > 0) {
        fwrite(prompt->bytes, 1, (size_t)prompt->len, stdout);
        fflush(stdout);
    }
    return leo_input_line();
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

#define LEO_LT_I64(x, y) ((x) < (y))
#define LEO_LT_F64(x, y) ((x) < (y))
#define LEO_LT_STR(x, y) (leo_str_cmp(x, y) < 0)

/* Type-specialized introsorts: insertion sort for small ranges,
   median-of-three quicksort inlined per type, heapsort when the
   recursion gets deep. No function pointers on the comparison path. */

#define LEO_ISORT_THRESHOLD 24

#define LEO_INTROSORT(NAME, T, LT)                                          \
static void NAME##_insertion(T* a, int64_t n) {                             \
    for (int64_t i = 1; i < n; i++) {                                       \
        T v = a[i];                                                          \
        int64_t j = i - 1;                                                   \
        while (j >= 0 && LT(a[j], v)) { a[j + 1] = a[j]; j--; }             \
        a[j + 1] = v;                                                        \
    }                                                                        \
}                                                                            \
static void NAME##_sift(T* a, int64_t start, int64_t end) {                  \
    int64_t root = start;                                                    \
    while (root * 2 + 1 <= end) {                                            \
        int64_t child = root * 2 + 1;                                         \
        if (child + 1 <= end && LT(a[child], a[child + 1])) child++;         \
        if (LT(a[root], a[child])) {                                          \
            T t = a[root]; a[root] = a[child]; a[child] = t;                  \
            root = child;                                                     \
        } else return;                                                        \
    }                                                                        \
}                                                                            \
static void NAME##_heapsort(T* a, int64_t n) {                               \
    for (int64_t start = (n - 2) / 2; start >= 0; start--)                    \
        NAME##_sift(a, start, n - 1);                                          \
    for (int64_t end = n - 1; end > 0; end--) {                               \
        T t = a[end]; a[end] = a[0]; a[0] = t;                                 \
        NAME##_sift(a, 0, end - 1);                                            \
    }                                                                        \
}                                                                            \
static void NAME##_recurse(T* a, int64_t n, int64_t depth) {                  \
    while (n > LEO_ISORT_THRESHOLD) {                                         \
        if (depth == 0) { NAME##_heapsort(a, n); return; }                     \
        depth--;                                                              \
        int64_t mid = n / 2;                                                  \
        T t;                                                                  \
        /* median of three to start */                                        \
        if (LT(a[mid], a[0])) { t = a[0]; a[0] = a[mid]; a[mid] = t; }         \
        if (LT(a[n - 1], a[0])) { t = a[0]; a[0] = a[n - 1]; a[n - 1] = t; }   \
        if (LT(a[n - 1], a[mid])) { t = a[mid]; a[mid] = a[n - 1]; a[n - 1] = t; } \
        T pivot = a[mid];                                                     \
        int64_t i = 0, j = n - 1;                                             \
        for (;;) {                                                            \
            while (LT(a[i], pivot)) i++;                                       \
            while (LT(pivot, a[j])) j--;                                      \
            if (i >= j) break;                                                 \
            t = a[i]; a[i] = a[j]; a[j] = t;                                   \
            i++; j--;                                                         \
        }                                                                    \
        /* recurse into the smaller half, loop on the larger */               \
        if (j + 1 < n - (j + 1)) {                                            \
            NAME##_recurse(a, j + 1, depth);                                   \
            a += j + 1;                                                       \
            n -= j + 1;                                                       \
        } else {                                                              \
            NAME##_recurse(a + j + 1, n - (j + 1), depth);                       \
            n = j + 1;                                                        \
        }                                                                    \
    }                                                                        \
    NAME##_insertion(a, n);                                                   \
}                                                                            \
void NAME(T* a, int64_t n) { NAME##_recurse(a, n, 48); }

LEO_INTROSORT(leo_isort_i64, int64_t, LEO_LT_I64)

/* Integer sorts skip comparisons entirely: LSD radix over sign-biased
   keys. Three passes of 11 bits each; linear time, cache-friendly. */
void leo_sort_i64_radix(int64_t* a, int64_t n) {
    if (n < 64) {
        leo_isort_i64_insertion(a, n);
        return;
    }
    /* Offset bias: keys become x - min, so the pass count tracks the
       actual data range. Values spanning zero compact to 31 bits. */
    int64_t mn = a[0], mx = a[0];
    for (int64_t i = 1; i < n; i++) {
        if (a[i] < mn) mn = a[i];
        if (a[i] > mx) mx = a[i];
    }
    uint64_t range = (uint64_t)mx - (uint64_t)mn;
    if (range == 0) return; /* all equal */
    int nbits = 0;
    while (nbits < 64 && (range >> nbits) != 0) nbits++;
    int passes = (nbits + 10) / 11;
    uint64_t* buf = (uint64_t*)malloc((size_t)n * sizeof(uint64_t));
    if (!buf) leo_panic("out of memory", "<runtime>", 0);
    uint64_t* src = (uint64_t*)a;
    for (int64_t i = 0; i < n; i++) src[i] -= (uint64_t)mn;
    for (int p = 0; p < passes; p++) {
        int shift = p * 11;
        int bits = nbits - shift;
        if (bits > 11) bits = 11;
        /* Strict ping-pong between the two buffers; an odd pass count
           pays one copy back at the end, an even count lands in a. */
        uint64_t* dst = (p % 2 == 0) ? buf : (uint64_t*)a;
        int count[2048];
        memset(count, 0, sizeof(count));
        uint64_t mask = (bits >= 64) ? ~0ULL : ((1ULL << bits) - 1);
        for (int64_t i = 0; i < n; i++) count[(src[i] >> shift) & mask]++;
        int64_t total = 0;
        for (int b = 0; b < (1 << bits); b++) {
            int64_t c = count[b];
            count[b] = total;
            total += c;
        }
        for (int64_t i = 0; i < n; i++) {
            int b = (int)((src[i] >> shift) & mask);
            dst[count[b]++] = src[i];
        }
        src = dst;
    }
    if (src != (uint64_t*)a) {
        memcpy(a, src, (size_t)n * sizeof(uint64_t));
        src = (uint64_t*)a;
    }
    for (int64_t i = 0; i < n; i++) a[i] = (int64_t)((uint64_t)a[i] + (uint64_t)mn);
    free(buf);
}

void leo_sort_i64(int64_t* a, int64_t n) {
    leo_sort_i64_radix(a, n);
}
LEO_INTROSORT(leo_sort_f64, double, LEO_LT_F64)
LEO_INTROSORT(leo_sort_str, leo_str*, LEO_LT_STR)


