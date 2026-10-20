/* Fang runtime implementation. See fang_rt.h for the surface. */

#define _POSIX_C_SOURCE 200809L
#define _DEFAULT_SOURCE

#include "fang_rt.h"

#include <unistd.h>
#include <dirent.h>
#include <time.h>
#include <sys/socket.h>
#include <netdb.h>
#include <sys/types.h>
#include <sys/stat.h>

static void fang_sb_str_raw(fang_sb* sb, const char* data, int64_t len);

/* ------------------------------------------------------------------ */
/* Panic                                                               */
/* ------------------------------------------------------------------ */

void fang_panic(const char* msg, const char* file, int line) {
    fprintf(stderr, "fang: runtime error in %s at line %d: %s\n", file, line, msg);
    exit(101);
}

/* ------------------------------------------------------------------ */
/* Strings                                                             */
/* ------------------------------------------------------------------ */

fang_str* fang_str_new(const char* data, int64_t len) {
    fang_str* s = (fang_str*)malloc(sizeof(fang_str) + (size_t)len + 1);
    if (!s) fang_panic("out of memory", "<runtime>", 0);
    s->rc = 1;
    s->len = len;
    s->cap = len;
    if (len > 0 && data) memcpy(s->bytes, data, (size_t)len);
    s->bytes[len] = '\0';
    return s;
}

fang_str* fang_str_concat(fang_str* a, fang_str* b) {
    int64_t n = a->len + b->len;
    fang_str* s = (fang_str*)malloc(sizeof(fang_str) + (size_t)n + 1);
    if (!s) fang_panic("out of memory", "<runtime>", 0);
    s->rc = 1;
    s->len = n;
    s->cap = n;
    if (a->len > 0) memcpy(s->bytes, a->bytes, (size_t)a->len);
    if (b->len > 0) memcpy(s->bytes + a->len, b->bytes, (size_t)b->len);
    s->bytes[n] = '\0';
    return s;
}

fang_str* fang_str_concat_assign(fang_str* a, fang_str* b) {
    if (a && b && a != b && a->rc == 1 && a->cap >= a->len + b->len) {
        if (b->len > 0) memcpy(a->bytes + a->len, b->bytes, (size_t)b->len);
        a->len += b->len;
        a->bytes[a->len] = '\0';
        return a;
    }
    if (!a) return fang_str_new(b ? b->bytes : "", b ? b->len : 0);
    if (!b) return a;
    int64_t n = a->len + b->len;
    int64_t cap = n * 2;
    fang_str* s = (fang_str*)malloc(sizeof(fang_str) + (size_t)cap + 1);
    if (!s) fang_panic("out of memory", "<runtime>", 0);
    s->rc = 1;
    s->len = n;
    s->cap = cap;
    if (a->len > 0) memcpy(s->bytes, a->bytes, (size_t)a->len);
    if (b->len > 0) memcpy(s->bytes + a->len, b->bytes, (size_t)b->len);
    s->bytes[n] = '\0';
    fang_str_decref(a);
    return s;
}

bool fang_str_eq(fang_str* a, fang_str* b) {
    return a == b || (a->len == b->len && memcmp(a->bytes, b->bytes, (size_t)a->len) == 0);
}

int fang_str_cmp(fang_str* a, fang_str* b) {
    int64_t n = a->len < b->len ? a->len : b->len;
    int c = n > 0 ? memcmp(a->bytes, b->bytes, (size_t)n) : 0;
    if (c != 0) return c < 0 ? -1 : 1;
    if (a->len == b->len) return 0;
    return a->len < b->len ? -1 : 1;
}

fang_str* fang_str_slice(fang_str* s, int64_t start, int64_t end) {
    if (start < 0) start += s->len;
    if (end < 0) end += s->len;
    if (start < 0) start = 0;
    if (end > s->len) end = s->len;
    if (start >= end) return fang_str_new("", 0);
    return fang_str_new(s->bytes + start, end - start);
}

fang_str* fang_str_char_at(fang_str* s, int64_t i) {
    if (i < 0) i += s->len;
    if (i < 0 || i >= s->len) fang_panic("string index out of bounds", __FILE__, __LINE__);
    return fang_str_new(s->bytes + i, 1);
}

int64_t fang_str_find(fang_str* s, int64_t from, fang_str* needle) {
    if (from < 0) from = 0;
    if (needle->len == 0) return from <= s->len ? from : -1;
    if (needle->len > s->len) return -1;
    for (int64_t i = from; i + needle->len <= s->len; i++) {
        if (memcmp(s->bytes + i, needle->bytes, (size_t)needle->len) == 0) return i;
    }
    return -1;
}

static char fang_ascii_upper(char c) {
    return (c >= 'a' && c <= 'z') ? (char)(c - 'a' + 'A') : c;
}
static char fang_ascii_lower(char c) {
    return (c >= 'A' && c <= 'Z') ? (char)(c - 'A' + 'a') : c;
}

fang_str* fang_str_upper(fang_str* s) {
    fang_str* out = fang_str_new(s->bytes, s->len);
    for (int64_t i = 0; i < out->len; i++) out->bytes[i] = fang_ascii_upper(out->bytes[i]);
    return out;
}

fang_str* fang_str_lower(fang_str* s) {
    fang_str* out = fang_str_new(s->bytes, s->len);
    for (int64_t i = 0; i < out->len; i++) out->bytes[i] = fang_ascii_lower(out->bytes[i]);
    return out;
}

fang_str* fang_str_replace(fang_str* s, fang_str* from, fang_str* to) {
    if (from->len == 0) return fang_str_new(s->bytes, s->len);
    fang_sb* sb = fang_sb_new();
    int64_t i = 0;
    while (i < s->len) {
        int64_t at = fang_str_find(s, i, from);
        if (at < 0) {
            fang_sb_str_raw(sb, s->bytes + i, s->len - i);
            break;
        }
        fang_sb_str_raw(sb, s->bytes + i, at - i);
        fang_sb_str_raw(sb, to->bytes, to->len);
        i = at + from->len;
    }
    return fang_sb_build(sb);
}

fang_str* fang_str_strip(fang_str* s) {
    int64_t a = 0, b = s->len;
    while (a < b && (s->bytes[a] == ' ' || s->bytes[a] == '\t' || s->bytes[a] == '\n' || s->bytes[a] == '\r')) a++;
    while (b > a && (s->bytes[b - 1] == ' ' || s->bytes[b - 1] == '\t' || s->bytes[b - 1] == '\n' || s->bytes[b - 1] == '\r')) b--;
    return fang_str_new(s->bytes + a, b - a);
}

bool fang_str_starts_with(fang_str* s, fang_str* prefix) {
    if (prefix->len > s->len) return false;
    return memcmp(s->bytes, prefix->bytes, (size_t)prefix->len) == 0;
}

bool fang_str_ends_with(fang_str* s, fang_str* suffix) {
    if (suffix->len > s->len) return false;
    return memcmp(s->bytes + s->len - suffix->len, suffix->bytes, (size_t)suffix->len) == 0;
}

fang_str* fang_str_from_i64(int64_t v) {
    char buf[32];
    int n = snprintf(buf, sizeof buf, "%lld", (long long)v);
    return fang_str_new(buf, n);
}

fang_str* fang_str_from_f64(double v) {
    char buf[40];
    /* shortest representation that round-trips */
    for (int prec = 1; prec <= 17; prec++) {
        int n = snprintf(buf, sizeof buf, "%.*g", prec, v);
        if (strtod(buf, NULL) == v) return fang_str_new(buf, n);
    }
    int n = snprintf(buf, sizeof buf, "%g", v);
    return fang_str_new(buf, n);
}

fang_str* fang_str_from_bool(bool v) {
    return fang_str_new(v ? "true" : "false", v ? 4 : 5);
}

int64_t fang_str_to_i64(fang_str* s) {
    if (s->len == 0) fang_panic("to_int: the string is empty", __FILE__, __LINE__);
    char* end = NULL;
    long long v = strtoll(s->bytes, &end, 10);
    if (end != s->bytes + s->len)
        fang_panic("to_int: not a whole number", __FILE__, __LINE__);
    return (int64_t)v;
}

double fang_str_to_f64(fang_str* s) {
    if (s->len == 0) fang_panic("to_float: the string is empty", __FILE__, __LINE__);
    char* end = NULL;
    double v = strtod(s->bytes, &end);
    if (end != s->bytes + s->len)
        fang_panic("to_float: not a number", __FILE__, __LINE__);
    return v;
}

uint64_t fang_str_hash(fang_str* s) {
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

void fang_print_str(fang_str* s) {
    fwrite(s->bytes, 1, (size_t)s->len, stdout);
}
void fang_print_i64(int64_t v) {
    printf("%lld", (long long)v);
}
void fang_print_f64(double v) {
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
void fang_print_bool(bool v) {
    fputs(v ? "true" : "false", stdout);
}
void fang_print_ptr(void* v) {
    printf("%p", v);
}
fang_str* fang_debug_str(void* v, int kind);

static void fang_debug_sb(fang_sb* sb, void* item, int item_kind);

/* item_kind: 0=str(ptr), 1=int64, 2=f64, 3=bool, 4=debug(list) */
static void fang_debug_sb(fang_sb* sb, void* item, int item_kind) {
    switch (item_kind) {
    case 0: {
        fang_str* s = (fang_str*)item;
        fang_sb_str_raw(sb, s ? s->bytes : "", s ? s->len : 0);
        break;
    }
    case 1: fang_sb_i64(sb, *(int64_t*)item); break;
    case 2: fang_sb_f64(sb, *(double*)item); break;
    case 3: fang_sb_bool(sb, *(bool*)item); break;
    default: break;
    }
}

fang_str* fang_debug_str(void* v, int kind) {
    fang_sb* sb = fang_sb_new();
    if (kind == 6) {
        /* dict[str, int]-style: printed as {k: v}; only the common
           stampings are rendered; anything else shows its pointer. */
        fang_sb_str_raw(sb, "{...}", 5);
        return fang_sb_build(sb);
    }
    /* All Fang lists share the {rc, len, cap, elems} header. */
    struct fang_dbg_list {
        int64_t rc, len, cap;
        void* elems;
    }* l = (struct fang_dbg_list*)v;
    int64_t n = l ? l->len : 0;
    int width = (kind == 5) ? 8 : 8; /* int64/f64/ptr slots are 8 bytes */
    const char* open = (kind == 5) ? "(" : "[";
    const char* close = (kind == 5) ? ")" : "]";
    fang_sb_str_raw(sb, open, 1);
    for (int64_t i = 0; i < n; i++) {
        if (i > 0) fang_sb_str_raw(sb, ", ", 2);
        char* slot = (char*)l->elems + i * width;
        int item_kind;
        if (kind == 0) item_kind = 0;          /* list[str] */
        else if (kind == 1) item_kind = 1;     /* list[int] */
        else if (kind == 2) item_kind = 2;     /* list[float] */
        else if (kind == 4) item_kind = 3;     /* list[bool] */
        else item_kind = 0;                     /* ptr-ish: best effort */
        if (kind == 0) {
            fang_str* s = *(fang_str**)slot;
            fang_sb_str_raw(sb, "'", 1);
            fang_sb_str_raw(sb, s ? s->bytes : "", s ? s->len : 0);
            fang_sb_str_raw(sb, "'", 1);
        } else {
            fang_debug_sb(sb, slot, item_kind);
        }
    }
    fang_sb_str_raw(sb, close, 1);
    return fang_sb_build(sb);
}

void fang_print_debug(void* v, int kind) {
    fang_str* s = fang_debug_str(v, kind);
    fwrite(s->bytes, 1, (size_t)s->len, stdout);
}

void fang_print_sep(void) {
    putchar(' ');
}
void fang_print_nl(void) {
    putchar('\n');
    fflush(stdout);
}

/* ------------------------------------------------------------------ */
/* raise / try / except                                                */
/* ------------------------------------------------------------------ */

void fang_raise(fang_str* value) {
    extern fang_handler* fang_handler_top;
    if (!fang_handler_top) {
        fprintf(stderr, "fang: uncaught error: ");
        fwrite(value->bytes, 1, (size_t)value->len, stderr);
        fputc('\n', stderr);
        exit(1);
    }
    fang_handler_top->value = value;
    longjmp(fang_handler_top->buf, 1);
}

fang_handler* fang_handler_top = NULL;

/* ------------------------------------------------------------------ */
/* String builder                                                      */
/* ------------------------------------------------------------------ */

fang_sb* fang_sb_new(void) {
    fang_sb* sb = (fang_sb*)malloc(sizeof(fang_sb));
    sb->cap = 32;
    sb->len = 0;
    sb->data = (char*)malloc(32);
    return sb;
}

static void fang_sb_grow(fang_sb* sb, int64_t extra) {
    if (sb->len + extra + 1 <= sb->cap) return;
    while (sb->len + extra + 1 > sb->cap) sb->cap *= 2;
    sb->data = (char*)realloc(sb->data, (size_t)sb->cap);
}

void fang_sb_str_raw(fang_sb* sb, const char* data, int64_t len) {
    fang_sb_grow(sb, len);
    memcpy(sb->data + sb->len, data, (size_t)len);
    sb->len += len;
}

void fang_sb_str(fang_sb* sb, fang_str* s) {
    fang_sb_str_raw(sb, s->bytes, s->len);
}
void fang_sb_i64(fang_sb* sb, int64_t v) {
    char buf[32];
    int n = snprintf(buf, sizeof buf, "%lld", (long long)v);
    fang_sb_str_raw(sb, buf, n);
}
void fang_sb_f64(fang_sb* sb, double v) {
    char buf[40];
    for (int prec = 1; prec <= 17; prec++) {
        int n = snprintf(buf, sizeof buf, "%.*g", prec, v);
        if (strtod(buf, NULL) == v) {
            fang_sb_str_raw(sb, buf, n);
            return;
        }
    }
    fang_sb_str_raw(sb, buf, snprintf(buf, sizeof buf, "%g", v));
}
void fang_sb_bool(fang_sb* sb, bool v) {
    fang_sb_str_raw(sb, v ? "true" : "false", v ? 4 : 5);
}

fang_str* fang_sb_build(fang_sb* sb) {
    fang_str* s = fang_str_new(sb->data, sb->len);
    free(sb->data);
    free(sb);
    return s;
}

/* ------------------------------------------------------------------ */
/* OS, filesystem, time                                                */
/* ------------------------------------------------------------------ */

static int g_argc = 0;
static char** g_argv = NULL;

void fang_os_init(int argc, char** argv) {
    g_argc = argc;
    g_argv = argv;
}

int64_t fang_os_argc(void) {
    return g_argc;
}

fang_str* fang_os_arg(int64_t i) {
    if (i < 0 || i >= g_argc) fang_panic("os.args index out of bounds", __FILE__, __LINE__);
    int64_t len = (int64_t)strlen(g_argv[i]);
    return fang_str_new(g_argv[i], len);
}

fang_str* fang_os_env(fang_str* name) {
    const char* v = getenv(name->bytes);
    if (!v) return fang_str_new("", 0);
    return fang_str_new(v, (int64_t)strlen(v));
}

fang_str* fang_os_getcwd(void) {
    char buf[4096];
    if (!getcwd(buf, sizeof buf)) fang_panic("getcwd failed", __FILE__, __LINE__);
    return fang_str_new(buf, (int64_t)strlen(buf));
}

int64_t fang_os_system(fang_str* cmd) {
    return (int64_t)system(cmd->bytes);
}

void fang_os_exit(int64_t code) {
    exit((int)code);
}

fang_str* fang_fs_read_file(fang_str* path) {
    FILE* f = fopen(path->bytes, "rb");
    if (!f) {
        fprintf(stderr, "fang: cannot open '%s' for reading\n", path->bytes);
        fang_panic("fs.read_file failed", __FILE__, __LINE__);
    }
    fseek(f, 0, SEEK_END);
    long size = ftell(f);
    fseek(f, 0, SEEK_SET);
    fang_str* s = (fang_str*)malloc(sizeof(fang_str) + (size_t)size + 1);
    if (!s) fang_panic("out of memory", "<runtime>", 0);
    s->rc = 1;
    s->len = size;
    s->cap = size;
    if (size > 0 && fread(s->bytes, 1, (size_t)size, f) != (size_t)size) {
        fclose(f);
        fang_panic("fs.read_file failed mid-read", __FILE__, __LINE__);
    }
    s->bytes[size] = '\0';
    fclose(f);
    return s;
}

void fang_fs_write_file(fang_str* path, fang_str* data) {
    FILE* f = fopen(path->bytes, "wb");
    if (!f) {
        fprintf(stderr, "fang: cannot open '%s' for writing\n", path->bytes);
        fang_panic("fs.write_file failed", __FILE__, __LINE__);
    }
    if (data->len > 0) fwrite(data->bytes, 1, (size_t)data->len, f);
    fclose(f);
}

void fang_fs_append_file(fang_str* path, fang_str* data) {
    FILE* f = fopen(path->bytes, "ab");
    if (!f) {
        fprintf(stderr, "fang: cannot open '%s' for appending\n", path->bytes);
        fang_panic("fs.append_file failed", __FILE__, __LINE__);
    }
    if (data->len > 0) fwrite(data->bytes, 1, (size_t)data->len, f);
    fclose(f);
}

bool fang_fs_exists(fang_str* path) {
    return access(path->bytes, F_OK) == 0;
}

fang_str* fang_fs_list_dir(fang_str* path) {
    /* Newline-joined entry names; the std module splits it. */
    fang_sb* sb = fang_sb_new();
    DIR* d = opendir(path->bytes);
    if (!d) {
        fprintf(stderr, "fang: cannot list directory '%s'\n", path->bytes);
        fang_panic("fs.list_dir failed", __FILE__, __LINE__);
    }
    struct dirent* e;
    bool first = true;
    while ((e = readdir(d)) != NULL) {
        if (strcmp(e->d_name, ".") == 0 || strcmp(e->d_name, "..") == 0) continue;
        if (!first) fang_sb_str_raw(sb, "\n", 1);
        first = false;
        fang_sb_str_raw(sb, e->d_name, (int64_t)strlen(e->d_name));
    }
    closedir(d);
    return fang_sb_build(sb);
}

void fang_fs_mkdir(fang_str* path) {
    if (mkdir(path->bytes, 0755) != 0) {
        fprintf(stderr, "fang: cannot create directory '%s'\n", path->bytes);
        fang_panic("fs.mkdir failed", __FILE__, __LINE__);
    }
}

void fang_fs_remove(fang_str* path) {
    if (remove(path->bytes) != 0) {
        fprintf(stderr, "fang: cannot remove '%s'\n", path->bytes);
        fang_panic("fs.remove failed", __FILE__, __LINE__);
    }
}

double fang_time_now(void) {
    struct timespec ts;
    clock_gettime(CLOCK_REALTIME, &ts);
    return (double)ts.tv_sec + (double)ts.tv_nsec * 1e-9;
}

double fang_time_clock(void) {
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return (double)ts.tv_sec + (double)ts.tv_nsec * 1e-9;
}

void fang_time_sleep(double seconds) {
    struct timespec ts;
    ts.tv_sec = (time_t)seconds;
    ts.tv_nsec = (long)((seconds - (double)ts.tv_sec) * 1e9);
    nanosleep(&ts, NULL);
}

fang_str* fang_input_prompt(fang_str* prompt) {
    if (prompt && prompt->len > 0) {
        fwrite(prompt->bytes, 1, (size_t)prompt->len, stdout);
        fflush(stdout);
    }
    return fang_input_line();
}

fang_str* fang_input_line(void) {
    char* line = NULL;
    size_t cap = 0;
    ssize_t n = getline(&line, &cap, stdin);
    if (n < 0) {
        free(line);
        return fang_str_new("", 0);
    }
    while (n > 0 && (line[n - 1] == '\n' || line[n - 1] == '\r')) n--;
    fang_str* s = fang_str_new(line, n);
    free(line);
    return s;
}

/* ------------------------------------------------------------------ */
/* Networking                                                          */
/* ------------------------------------------------------------------ */

void* fang_tcp_listen(int64_t port) {
    int fd = socket(AF_INET, SOCK_STREAM, 0);
    if (fd < 0) fang_panic("socket creation failed", __FILE__, __LINE__);
    int yes = 1;
    setsockopt(fd, SOL_SOCKET, SO_REUSEADDR, &yes, sizeof yes);
    struct sockaddr_in addr;
    memset(&addr, 0, sizeof addr);
    addr.sin_family = AF_INET;
    addr.sin_addr.s_addr = htonl(INADDR_ANY);
    addr.sin_port = htons((uint16_t)port);
    if (bind(fd, (struct sockaddr*)&addr, sizeof addr) != 0) {
        close(fd);
        fang_panic("cannot bind to port (is it already in use?)", __FILE__, __LINE__);
    }
    if (listen(fd, 128) != 0) {
        close(fd);
        fang_panic("listen failed", __FILE__, __LINE__);
    }
    return (void*)(intptr_t)fd;
}

void* fang_tcp_accept(void* srv) {
    int fd = (int)(intptr_t)srv;
    int client = accept(fd, NULL, NULL);
    if (client < 0) fang_panic("accept failed", __FILE__, __LINE__);
    return (void*)(intptr_t)client;
}

void* fang_net_resolve(fang_str* host, int64_t port) {
    char portbuf[16];
    snprintf(portbuf, sizeof portbuf, "%lld", (long long)port);
    struct addrinfo hints, *res = NULL;
    memset(&hints, 0, sizeof hints);
    hints.ai_family = AF_INET;
    hints.ai_socktype = SOCK_STREAM;
    int rc = getaddrinfo(host->bytes, portbuf, &hints, &res);
    if (rc != 0 || !res) {
        fprintf(stderr, "fang: cannot resolve '%s'\n", host->bytes);
        fang_panic("net resolve failed", __FILE__, __LINE__);
    }
    int fd = socket(res->ai_family, res->ai_socktype, res->ai_protocol);
    if (fd < 0) {
        freeaddrinfo(res);
        fang_panic("socket creation failed", __FILE__, __LINE__);
    }
    if (connect(fd, res->ai_addr, res->ai_addrlen) != 0) {
        close(fd);
        freeaddrinfo(res);
        fprintf(stderr, "fang: cannot connect to '%s' port %lld\n", host->bytes, (long long)port);
        fang_panic("net connect failed", __FILE__, __LINE__);
    }
    freeaddrinfo(res);
    return (void*)(intptr_t)fd;
}

int64_t fang_tcp_send(void* sock, fang_str* data) {
    int fd = (int)(intptr_t)sock;
    int64_t total = 0;
    while (total < data->len) {
        ssize_t n = send(fd, data->bytes + total, (size_t)(data->len - total), 0);
        if (n <= 0) fang_panic("tcp send failed", __FILE__, __LINE__);
        total += n;
    }
    return total;
}

fang_str* fang_tcp_recv_all(void* sock) {
    int fd = (int)(intptr_t)sock;
    fang_sb* sb = fang_sb_new();
    char buf[16384];
    for (;;) {
        ssize_t n = recv(fd, buf, sizeof buf, 0);
        if (n < 0) fang_panic("tcp receive failed", __FILE__, __LINE__);
        if (n == 0) break;
        fang_sb_str_raw(sb, buf, n);
    }
    return fang_sb_build(sb);
}

/* Read one HTTP request: headers plus any Content-Length body. Returns
   once the request is complete; does not wait for the peer to close. */
fang_str* fang_tcp_recv_http(void* sock) {
    int fd = (int)(intptr_t)sock;
    fang_sb* sb = fang_sb_new();
    char buf[16384];
    int64_t header_end = -1;
    int64_t content_length = 0;
    for (;;) {
        if (header_end < 0 || (int64_t)sb->len < header_end + 4 + content_length) {
            ssize_t n = recv(fd, buf, sizeof buf, 0);
            if (n < 0) fang_panic("tcp receive failed", __FILE__, __LINE__);
            if (n == 0) break;
            fang_sb_str_raw(sb, buf, n);
        }
        char* data = (char*)sb->data;
        int64_t len = (int64_t)sb->len;
        if (header_end < 0) {
            for (int64_t i = 0; i + 3 < len; i++) {
                if (data[i] == '\r' && data[i + 1] == '\n'
                    && data[i + 2] == '\r' && data[i + 3] == '\n') {
                    header_end = i;
                    break;
                }
            }
            if (header_end < 0) continue;
            /* Find Content-Length in the header block (case-insensitive). */
            for (int64_t i = 0; i + 15 < header_end; i++) {
                if ((data[i] == 'c' || data[i] == 'C')
                    && strncasecmp(data + i, "content-length:", 15) == 0) {
                    int64_t j = i + 15;
                    while (j < header_end && (data[j] == ' ' || data[j] == '\t')) j++;
                    content_length = strtoll(data + j, NULL, 10);
                    break;
                }
            }
        }
        if (len >= header_end + 4 + content_length) break;
    }
    return fang_sb_build(sb);
}

void fang_tcp_close(void* sock) {
    close((int)(intptr_t)sock);
}

/* ------------------------------------------------------------------ */
/* Sorting                                                             */
/* ------------------------------------------------------------------ */

#define FANG_LT_I64(x, y) ((x) < (y))
#define FANG_LT_F64(x, y) ((x) < (y))
#define FANG_LT_STR(x, y) (fang_str_cmp(x, y) < 0)

/* Type-specialized introsorts: insertion sort for small ranges,
   median-of-three quicksort inlined per type, heapsort when the
   recursion gets deep. No function pointers on the comparison path. */

#define FANG_ISORT_THRESHOLD 24

#define FANG_INTROSORT(NAME, T, LT)                                          \
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
    while (n > FANG_ISORT_THRESHOLD) {                                         \
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

FANG_INTROSORT(fang_isort_i64, int64_t, FANG_LT_I64)

/* Integer sorts skip comparisons entirely: LSD radix over sign-biased
   keys. Three passes of 11 bits each; linear time, cache-friendly. */
void fang_sort_i64_radix(int64_t* a, int64_t n) {
    if (n < 64) {
        fang_isort_i64_insertion(a, n);
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
    if (!buf) fang_panic("out of memory", "<runtime>", 0);
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

void fang_sort_i64(int64_t* a, int64_t n) {
    fang_sort_i64_radix(a, n);
}
FANG_INTROSORT(fang_sort_f64, double, FANG_LT_F64)
FANG_INTROSORT(fang_sort_str, fang_str*, FANG_LT_STR)


