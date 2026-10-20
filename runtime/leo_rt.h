#ifndef LEO_RT_H
#define LEO_RT_H

#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdbool.h>

/* ------------------------------------------------------------------ */
/* Reference counting. Every refcounted object starts with int64_t rc */
/* as its first field. LEO_RC_STATIC marks immortal objects.          */
/* ------------------------------------------------------------------ */

#define LEO_RC_STATIC (-1)

typedef struct leo_obj { int64_t rc; } leo_obj;

static inline void leo_incref(void* p) {
    leo_obj* o = (leo_obj*)p;
    if (o && o->rc != LEO_RC_STATIC) o->rc++;
}

/* ------------------------------------------------------------------ */
/* Strings: length-prefixed, NUL-terminated, immutable, refcounted.    */
/* ------------------------------------------------------------------ */

typedef struct leo_str {
    int64_t rc;
    int64_t len;
    /* allocated capacity; len never exceeds it */
    int64_t cap;
    char bytes[];
} leo_str;

static inline void leo_str_decref(leo_str* s) {
    if (s && s->rc != LEO_RC_STATIC && --s->rc == 0) free(s);
}

leo_str* leo_str_new(const char* data, int64_t len);
leo_str* leo_str_concat(leo_str* a, leo_str* b);
/* Consumes a's reference: appends in place when a is uniquely held and
   has capacity, otherwise allocates with headroom and releases a. */
leo_str* leo_str_concat_assign(leo_str* a, leo_str* b);
bool leo_str_eq(leo_str* a, leo_str* b);
int leo_str_cmp(leo_str* a, leo_str* b);
leo_str* leo_str_slice(leo_str* s, int64_t start, int64_t end);
int64_t leo_str_find(leo_str* s, int64_t from, leo_str* needle);
leo_str* leo_str_upper(leo_str* s);
leo_str* leo_str_lower(leo_str* s);
leo_str* leo_str_replace(leo_str* s, leo_str* from, leo_str* to);
leo_str* leo_str_strip(leo_str* s);
bool leo_str_starts_with(leo_str* s, leo_str* prefix);
bool leo_str_ends_with(leo_str* s, leo_str* suffix);
leo_str* leo_str_from_i64(int64_t v);
leo_str* leo_str_from_f64(double v);
leo_str* leo_str_from_bool(bool v);
int64_t leo_str_to_i64(leo_str* s);
double leo_str_to_f64(leo_str* s);
uint64_t leo_str_hash(leo_str* s);
leo_str* leo_str_char_at(leo_str* s, int64_t i);

/* Print: values then separators, newline at end. */
void leo_print_str(leo_str* s);
void leo_print_i64(int64_t v);
void leo_print_f64(double v);
void leo_print_bool(bool v);
void leo_print_ptr(void* v);
void leo_print_sep(void);
void leo_print_nl(void);

/* Panic: message with Leo file/line (mapped by #line directives). */
void leo_panic(const char* msg, const char* file, int line);

/* ------------------------------------------------------------------ */
/* raise / try / except: setjmp handler stack.                         */
/* ------------------------------------------------------------------ */

#include <setjmp.h>

typedef struct leo_handler {
    jmp_buf buf;
    leo_str* value;
    struct leo_handler* prev;
} leo_handler;

void leo_raise(leo_str* value);

extern leo_handler* leo_handler_top;

/* ------------------------------------------------------------------ */
/* String builder for f-strings.                                       */
/* ------------------------------------------------------------------ */

typedef struct leo_sb {
    char* data;
    int64_t len, cap;
} leo_sb;

leo_sb* leo_sb_new(void);
void leo_sb_str(leo_sb* sb, leo_str* s);
void leo_sb_i64(leo_sb* sb, int64_t v);
void leo_sb_f64(leo_sb* sb, double v);
void leo_sb_bool(leo_sb* sb, bool v);
leo_str* leo_sb_build(leo_sb* sb);

/* ------------------------------------------------------------------ */
/* OS, filesystem, time.                                               */
/* ------------------------------------------------------------------ */

void leo_os_init(int argc, char** argv);
int64_t leo_os_argc(void);
leo_str* leo_os_arg(int64_t i);
leo_str* leo_os_env(leo_str* name);
leo_str* leo_os_getcwd(void);
int64_t leo_os_system(leo_str* cmd);
void leo_os_exit(int64_t code);
leo_str* leo_fs_read_file(leo_str* path);
void leo_fs_write_file(leo_str* path, leo_str* data);
void leo_fs_append_file(leo_str* path, leo_str* data);
bool leo_fs_exists(leo_str* path);
leo_str* leo_fs_list_dir(leo_str* path);
void leo_fs_mkdir(leo_str* path);
void leo_fs_remove(leo_str* path);
double leo_time_now(void);
double leo_time_clock(void);
void leo_time_sleep(double seconds);
leo_str* leo_input_line(void);

/* ------------------------------------------------------------------ */
/* Networking: TCP + DNS.                                              */
/* ------------------------------------------------------------------ */

void* leo_net_resolve(leo_str* host, int64_t port);
int64_t leo_tcp_send(void* sock, leo_str* data);
leo_str* leo_tcp_recv_all(void* sock);
void leo_tcp_close(void* sock);

/* ------------------------------------------------------------------ */
/* Closures: a function pointer plus an environment. Refcounted.      */
/* ------------------------------------------------------------------ */

typedef void (*leo_destroy_fn)(void* env);

typedef struct leo_closure {
    int64_t rc;
    void* env;
    void* fnptr;
    leo_destroy_fn destroy;
} leo_closure;

static inline leo_closure* leo_closure_new(void* env, void* fnptr, leo_destroy_fn destroy) {
    leo_closure* c = (leo_closure*)malloc(sizeof(leo_closure));
    if (!c) leo_panic("out of memory", "<runtime>", 0);
    c->rc = 1;
    c->env = env;
    c->fnptr = fnptr;
    c->destroy = destroy;
    return c;
}

static inline void leo_closure_decref(leo_closure* c) {
    if (c && c->rc != LEO_RC_STATIC && --c->rc == 0) {
        if (c->destroy && c->env) c->destroy(c->env);
        free(c);
    }
}

/* ------------------------------------------------------------------ */
/* Sorting for the stamped list types.                                 */
/* ------------------------------------------------------------------ */

void leo_sort_i64(int64_t* base, int64_t n);
void leo_sort_f64(double* base, int64_t n);
void leo_sort_str(leo_str** base, int64_t n);

/* ------------------------------------------------------------------ */
/* Collection templates, stamped by the codegen per instantiation.   */
/* SFX: type suffix. T: element type. EQ: equality expression.         */
/* INC / DEC: element refcount hooks, noop for primitives.            */
/* ------------------------------------------------------------------ */

#define LEO_LIST_DEFINE(SFX, T, EQ, INC, DEC)                                 \
  typedef struct { int64_t rc, len, cap; T* elems; } leo_list_##SFX;          \
  static leo_list_##SFX* leo_list_##SFX##_new(void) {                         \
    leo_list_##SFX* l = (leo_list_##SFX*)malloc(sizeof(*l));                  \
    l->rc = 1; l->len = 0; l->cap = 0; l->elems = NULL; return l;             \
  }                                                                           \
  static void leo_list_##SFX##_destroy(leo_list_##SFX* l) {                   \
    for (int64_t i = 0; i < l->len; i++) DEC(l->elems[i]);                    \
    free(l->elems); free(l);                                                   \
  }                                                                           \
  static inline void leo_list_##SFX##_decref(leo_list_##SFX* l) {             \
    if (l && l->rc != LEO_RC_STATIC && --l->rc == 0) leo_list_##SFX##_destroy(l); \
  }                                                                           \
  static void leo_list_##SFX##_push(leo_list_##SFX* l, T v) {                 \
    if (l->len == l->cap) {                                                   \
      l->cap = l->cap ? l->cap * 2 : 4;                                       \
      l->elems = (T*)realloc(l->elems, (size_t)l->cap * sizeof(T));           \
    }                                                                         \
    l->elems[l->len++] = v;                                                   \
  }                                                                           \
  static T leo_list_##SFX##_pop(leo_list_##SFX* l) {                          \
    if (l->len == 0) leo_panic("pop from an empty list", __FILE__, __LINE__); \
    return l->elems[--l->len];                                                \
  }                                                                           \
  static void leo_list_##SFX##_insert(leo_list_##SFX* l, int64_t i, T v) {    \
    if (i < 0) i += l->len;                                                   \
    if (i < 0 || i > l->len) leo_panic("insert index out of bounds", __FILE__, __LINE__); \
    leo_list_##SFX##_push(l, v);                                              \
    for (int64_t j = l->len - 1; j > i; j--) l->elems[j] = l->elems[j - 1];   \
    l->elems[i] = v;                                                          \
  }                                                                           \
  static void leo_list_##SFX##_remove_at(leo_list_##SFX* l, int64_t i) {      \
    if (i < 0) i += l->len;                                                   \
    if (i < 0 || i >= l->len) leo_panic("remove_at index out of bounds", __FILE__, __LINE__); \
    DEC(l->elems[i]);                                                         \
    for (int64_t j = i; j < l->len - 1; j++) l->elems[j] = l->elems[j + 1];   \
    l->len--;                                                                 \
  }                                                                           \
  static bool leo_list_##SFX##_contains(leo_list_##SFX* l, T v) {             \
    for (int64_t i = 0; i < l->len; i++) if (EQ(l->elems[i], v)) return true; \
    return false;                                                             \
  }                                                                           \
  static int64_t leo_list_##SFX##_index_of(leo_list_##SFX* l, T v) {          \
    for (int64_t i = 0; i < l->len; i++) if (EQ(l->elems[i], v)) return i;    \
    return -1;                                                                \
  }                                                                           \
  static void leo_list_##SFX##_reverse(leo_list_##SFX* l) {                   \
    for (int64_t i = 0, j = l->len - 1; i < j; i++, j--) {                    \
      T t = l->elems[i]; l->elems[i] = l->elems[j]; l->elems[j] = t;          \
    }                                                                         \
  }                                                                           \
  static T leo_list_##SFX##_get(leo_list_##SFX* l, int64_t i) {               \
    if (i < 0) i += l->len;                                                   \
    if (i < 0 || i >= l->len) leo_panic("list index out of bounds", __FILE__, __LINE__); \
    return l->elems[i];                                                       \
  }                                                                           \
  static void leo_list_##SFX##_set(leo_list_##SFX* l, int64_t i, T v) {       \
    if (i < 0) i += l->len;                                                   \
    if (i < 0 || i >= l->len) leo_panic("list index out of bounds", __FILE__, __LINE__); \
    DEC(l->elems[i]);                                                         \
    l->elems[i] = v;                                                          \
  }                                                                           \
  static leo_list_##SFX* leo_list_##SFX##_slice(leo_list_##SFX* l, int64_t a, int64_t b) { \
    if (a < 0) a += l->len;                                                   \
    if (b < 0) b += l->len;                                                   \
    if (a < 0) a = 0;                                                         \
    if (b > l->len) b = l->len;                                               \
    leo_list_##SFX* out = leo_list_##SFX##_new();                             \
    for (int64_t i = a; i < b; i++) { INC(l->elems[i]); leo_list_##SFX##_push(out, l->elems[i]); } \
    return out;                                                               \
  }                                                                           \
  static leo_list_##SFX* leo_list_##SFX##_concat(leo_list_##SFX* x, leo_list_##SFX* y) { \
    leo_list_##SFX* out = leo_list_##SFX##_new();                             \
    for (int64_t i = 0; i < x->len; i++) { INC(x->elems[i]); leo_list_##SFX##_push(out, x->elems[i]); } \
    for (int64_t i = 0; i < y->len; i++) { INC(y->elems[i]); leo_list_##SFX##_push(out, y->elems[i]); } \
    return out;                                                               \
  }

#define LEO_NOOP(x) ((void)0)
#define LEO_EQ_I64(a, b) ((a) == (b))
#define LEO_EQ_F64(a, b) ((a) == (b))
#define LEO_EQ_B8(a, b) ((a) == (b))
#define LEO_EQ_PTR(a, b) ((a) == (b))
#define LEO_EQ_STR(a, b) leo_str_eq((a), (b))
#define LEO_EQ_NEVER(a, b) (false)
#define LEO_INC_STR(x) leo_incref(x)
#define LEO_DEC_STR(x) leo_str_decref(x)

/* Tuples: plain value pairs. */
#define LEO_TUPLE_DEFINE(SFXA, SFXB, A, B)                                    \
  typedef struct { A a; B b; } leo_tuple_##SFXA##_##SFXB;

/* Dicts: open addressing, insertion ordered via probe sequence scan.
   KHASH: hash expr, KEQ: equality expr. Requires the K and V list
   stamps and the tuple stamp to already exist in this translation
   unit; the codegen guarantees that ordering. */
#define LEO_DICT_DEFINE(KS, VS, K, V, KHASH, KEQ, DEC_K, DEC_V, KINC, VINC)   \
  typedef struct { int64_t order; K key; V val; } leo_dslot_##KS##_##VS; \
  typedef struct { int64_t rc, len, cap; leo_dslot_##KS##_##VS* slots; } leo_dict_##KS##_##VS; \
  static leo_dict_##KS##_##VS* leo_dict_##KS##_##VS##_new(void) {             \
    leo_dict_##KS##_##VS* d = (leo_dict_##KS##_##VS*)malloc(sizeof(*d));     \
    d->rc = 1; d->len = 0; d->cap = 8;                                        \
    d->slots = (leo_dslot_##KS##_##VS*)calloc(8, sizeof(*d->slots));           \
    for (int64_t i = 0; i < 8; i++) d->slots[i].order = -1;                     \
    return d;                                                                 \
  }                                                                           \
  static void leo_dict_##KS##_##VS##_destroy(leo_dict_##KS##_##VS* d) {       \
    for (int64_t i = 0; i < d->cap; i++)                                      \
      if (d->slots[i].order >= 0) { DEC_K(d->slots[i].key); DEC_V(d->slots[i].val); } \
    free(d->slots); free(d);                                                   \
  }                                                                           \
  static inline void leo_dict_##KS##_##VS##_decref(leo_dict_##KS##_##VS* d) { \
    if (d && d->rc != LEO_RC_STATIC && --d->rc == 0) leo_dict_##KS##_##VS##_destroy(d); \
  }                                                                           \
  static void leo_dict_##KS##_##VS##_grow(leo_dict_##KS##_##VS* d) {          \
    int64_t ncap = d->cap * 2;                                                \
    leo_dslot_##KS##_##VS* ns = (leo_dslot_##KS##_##VS*)calloc((size_t)ncap, sizeof(*ns)); \
    for (int64_t i = 0; i < ncap; i++) ns[i].order = -1;                        \
    int64_t next = 0;                                                          \
    for (int64_t i = 0; i < d->cap; i++) {                                     \
      if (d->slots[i].order >= 0) {                                           \
        uint64_t h = (uint64_t)(KHASH(d->slots[i].key));                        \
        int64_t j = (int64_t)(h & (uint64_t)(ncap - 1));                       \
        while (ns[j].order != -1) j = (j + 1) & (ncap - 1);                    \
        ns[j] = d->slots[i]; ns[j].order = next++;                             \
      }                                                                       \
    }                                                                          \
    free(d->slots); d->slots = ns; d->cap = ncap;                              \
  }                                                                            \
  static void leo_dict_##KS##_##VS##_reserve(leo_dict_##KS##_##VS* d, int64_t n) { \
    while (d->cap * 8 < (n + 1) * 10) leo_dict_##KS##_##VS##_grow(d);         \
  }                                                                           \
  static void leo_dict_##KS##_##VS##_set(leo_dict_##KS##_##VS* d, K k, V v) { \
    /* Tombstones count toward the load factor so a churned table grows */   \
    /* instead of filling its probe space.                                   */ \
    if ((d->len + 1) * 10 >= d->cap * 8) leo_dict_##KS##_##VS##_grow(d);      \
    uint64_t h = (uint64_t)(KHASH(k));                                        \
    int64_t j = (int64_t)(h & (uint64_t)(d->cap - 1));                        \
    int64_t reuse = -1;                                                        \
    while (d->slots[j].order != -1) {                                          \
      if (d->slots[j].order == -2) {                                           \
        if (reuse < 0) reuse = j;                                             \
      } else if (KEQ(d->slots[j].key, k)) {                                    \
        DEC_V(d->slots[j].val);                                               \
        d->slots[j].val = v;                                                  \
        return;                                                               \
      }                                                                       \
      j = (j + 1) & (d->cap - 1);                                             \
    }                                                                         \
    if (reuse >= 0) {                                                         \
      KINC(k); d->slots[reuse].key = k;                                        \
      d->slots[reuse].val = v; d->slots[reuse].order = d->len++;               \
      return;                                                                 \
    }                                                                         \
    KINC(k); d->slots[j].key = k; d->slots[j].val = v;                        \
    d->slots[j].order = d->len++;                                              \
  }                                                                           \
  static bool leo_dict_##KS##_##VS##_contains(leo_dict_##KS##_##VS* d, K k) { \
    uint64_t h = (uint64_t)(KHASH(k));                                        \
    int64_t j = (int64_t)(h & (uint64_t)(d->cap - 1));                        \
    while (d->slots[j].order != -1) {                                          \
      if (d->slots[j].order != -2 && KEQ(d->slots[j].key, k)) return true;     \
      j = (j + 1) & (d->cap - 1);                                             \
    }                                                                         \
    return false;                                                             \
  }                                                                           \
  static V leo_dict_##KS##_##VS##_get(leo_dict_##KS##_##VS* d, K k, V def) {  \
    uint64_t h = (uint64_t)(KHASH(k));                                        \
    int64_t j = (int64_t)(h & (uint64_t)(d->cap - 1));                        \
    while (d->slots[j].order != -1) {                                          \
      if (d->slots[j].order != -2 && KEQ(d->slots[j].key, k)) return d->slots[j].val; \
      j = (j + 1) & (d->cap - 1);                                             \
    }                                                                         \
    return def;                                                               \
  }                                                                           \
  static bool leo_dict_##KS##_##VS##_remove(leo_dict_##KS##_##VS* d, K k) {   \
    uint64_t h = (uint64_t)(KHASH(k));                                        \
    int64_t j = (int64_t)(h & (uint64_t)(d->cap - 1));                        \
    while (d->slots[j].order != -1) {                                          \
      if (d->slots[j].order >= 0 && KEQ(d->slots[j].key, k)) {                 \
        DEC_K(d->slots[j].key); DEC_V(d->slots[j].val);                       \
        int64_t hole = d->slots[j].order;                                      \
        d->slots[j].order = -2;                                                 \
        for (int64_t i = 0; i < d->cap; i++)                                   \
          if (d->slots[i].order > hole) d->slots[i].order--;                   \
        d->len--;                                                             \
        return true;                                                           \
      }                                                                       \
      j = (j + 1) & (d->cap - 1);                                             \
    }                                                                         \
    return false;                                                             \
  }                                                                           \
  static leo_list_##KS* leo_dict_##KS##_##VS##_keys(leo_dict_##KS##_##VS* d) { \
    leo_list_##KS* out = leo_list_##KS##_new();                               \
    for (int64_t ord = 0; ord < d->len; ord++)                                 \
      for (int64_t i = 0; i < d->cap; i++)                                     \
        if (d->slots[i].order == ord) { KINC(d->slots[i].key); leo_list_##KS##_push(out, d->slots[i].key); } \
    return out;                                                               \
  }                                                                           \
  static leo_list_##VS* leo_dict_##KS##_##VS##_values(leo_dict_##KS##_##VS* d) { \
    leo_list_##VS* out = leo_list_##VS##_new();                                \
    for (int64_t ord = 0; ord < d->len; ord++)                                 \
      for (int64_t i = 0; i < d->cap; i++)                                     \
        if (d->slots[i].order == ord) { VINC(d->slots[i].val); leo_list_##VS##_push(out, d->slots[i].val); } \
    return out;                                                               \
  }                                                                           \
  static leo_list_t_##KS##_##VS* leo_dict_##KS##_##VS##_items(leo_dict_##KS##_##VS* d) { \
    leo_list_t_##KS##_##VS* out = leo_list_t_##KS##_##VS##_new();              \
    for (int64_t ord = 0; ord < d->len; ord++)                                 \
      for (int64_t i = 0; i < d->cap; i++)                                     \
        if (d->slots[i].order == ord) {                                         \
          leo_tuple_##KS##_##VS t = { d->slots[i].key, d->slots[i].val };      \
          KINC(t.a); VINC(t.b);                                                \
          leo_list_t_##KS##_##VS##_push(out, t);                               \
        }                                                                     \
    return out;                                                               \
  }

#define LEO_HASH_I64(k) ((uint64_t)((int64_t)(k) * 0x9E3779B97F4A7C15ULL))
#define LEO_HASH_STR(k) leo_str_hash((k))

#endif /* LEO_RT_H */
