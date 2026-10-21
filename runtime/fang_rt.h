#ifndef FANG_RT_H
#define FANG_RT_H

#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdbool.h>

/* ------------------------------------------------------------------ */
/* Reference counting. Every refcounted object starts with int64_t rc */
/* as its first field. FANG_RC_STATIC marks immortal objects.          */
/* ------------------------------------------------------------------ */

#define FANG_RC_STATIC (-1)

typedef struct fang_obj { int64_t rc; } fang_obj;

static inline void fang_incref(void* p) {
    fang_obj* o = (fang_obj*)p;
    if (o && o->rc != FANG_RC_STATIC) o->rc++;
}

/* ------------------------------------------------------------------ */
/* Strings: length-prefixed, NUL-terminated, immutable, refcounted.    */
/* ------------------------------------------------------------------ */

typedef struct fang_str {
    int64_t rc;
    int64_t len;
    /* allocated capacity; len never exceeds it */
    int64_t cap;
    char bytes[];
} fang_str;

static inline void fang_str_decref(fang_str* s) {
    if (s && s->rc != FANG_RC_STATIC && --s->rc == 0) free(s);
}

fang_str* fang_str_new(const char* data, int64_t len);
fang_str* fang_str_concat(fang_str* a, fang_str* b);
/* Consumes a's reference: appends in place when a is uniquely held and
   has capacity, otherwise allocates with headroom and releases a. */
fang_str* fang_str_concat_assign(fang_str* a, fang_str* b);
bool fang_str_eq(fang_str* a, fang_str* b);
int fang_str_cmp(fang_str* a, fang_str* b);
fang_str* fang_str_slice(fang_str* s, int64_t start, int64_t end);
int64_t fang_str_find(fang_str* s, int64_t from, fang_str* needle);
fang_str* fang_str_upper(fang_str* s);
fang_str* fang_str_lower(fang_str* s);
fang_str* fang_str_replace(fang_str* s, fang_str* from, fang_str* to);
fang_str* fang_str_strip(fang_str* s);
bool fang_str_starts_with(fang_str* s, fang_str* prefix);
bool fang_str_ends_with(fang_str* s, fang_str* suffix);
fang_str* fang_str_from_i64(int64_t v);
fang_str* fang_str_from_f64(double v);
fang_str* fang_str_from_bool(bool v);
int64_t fang_str_to_i64(fang_str* s);
double fang_str_to_f64(fang_str* s);
uint64_t fang_str_hash(fang_str* s);
fang_str* fang_str_char_at(fang_str* s, int64_t i);
int64_t fang_str_byte_at(fang_str* s, int64_t i);

/* Print: values then separators, newline at end. */
void fang_print_str(fang_str* s);
void fang_print_raw(fang_str* s);
/* Debug rendering used when a collection is printed: [1, 2, 3]. */
void fang_print_debug(void* v, int kind);
/* kind: 0 = list[str], 1 = list[int], 2 = list[float], 3 = list[ptr],
   4 = list[bool], 5 = tuple, 6 = dict. (ptr lists print each element's
   repr when it is a fang_str*.) */
fang_str* fang_debug_str(void* v, int kind);
void fang_print_i64(int64_t v);
void fang_print_f64(double v);
void fang_print_bool(bool v);
void fang_print_ptr(void* v);
void fang_print_sep(void);
void fang_print_nl(void);

/* Panic: message with Fang file/line (mapped by #line directives). */
void fang_panic(const char* msg, const char* file, int line);

/* ------------------------------------------------------------------ */
/* raise / try / except: setjmp handler stack.                         */
/* ------------------------------------------------------------------ */

#include <setjmp.h>

typedef struct fang_handler {
    jmp_buf buf;
    fang_str* value;
    struct fang_handler* prev;
} fang_handler;

void fang_raise(fang_str* value);

extern fang_handler* fang_handler_top;

/* ------------------------------------------------------------------ */
/* String builder for f-strings.                                       */
/* ------------------------------------------------------------------ */

typedef struct fang_sb {
    char* data;
    int64_t len, cap;
} fang_sb;

fang_sb* fang_sb_new(void);
void fang_sb_str(fang_sb* sb, fang_str* s);
void fang_sb_i64(fang_sb* sb, int64_t v);
void fang_sb_f64(fang_sb* sb, double v);
void fang_sb_bool(fang_sb* sb, bool v);
fang_str* fang_sb_build(fang_sb* sb);

/* ------------------------------------------------------------------ */
/* OS, filesystem, time.                                               */
/* ------------------------------------------------------------------ */

void fang_os_init(int argc, char** argv);
int64_t fang_os_argc(void);
fang_str* fang_os_arg(int64_t i);
fang_str* fang_os_env(fang_str* name);
fang_str* fang_os_getcwd(void);
int64_t fang_os_system(fang_str* cmd);
void fang_os_exit(int64_t code);
fang_str* fang_fs_read_file(fang_str* path);
void fang_fs_write_file(fang_str* path, fang_str* data);
void fang_fs_append_file(fang_str* path, fang_str* data);
bool fang_fs_exists(fang_str* path);
fang_str* fang_fs_list_dir(fang_str* path);
void fang_fs_mkdir(fang_str* path);
void fang_fs_remove(fang_str* path);
double fang_time_now(void);
double fang_time_clock(void);
void fang_time_sleep(double seconds);
fang_str* fang_input_line(void);
fang_str* fang_input_prompt(fang_str* prompt);

/* ------------------------------------------------------------------ */
/* Networking: TCP + DNS.                                              */
/* ------------------------------------------------------------------ */

void* fang_net_resolve(fang_str* host, int64_t port);
/* Server side: bind to a port and listen. */
void* fang_tcp_listen(int64_t port);
/* Blocking accept on a listening socket; returns the client fd. */
void* fang_tcp_accept(void* srv);
/* Read one HTTP request: headers plus Content-Length body. */
fang_str* fang_tcp_recv_http(void* sock);
/* LLM inference kernels. Quant types: 0 F32, 6 Q5_0, 8 Q8_0, 12 Q4_K,
   14 Q6_K. Activations are quantized in C to Q8_0 per call and dotted
   against the quantized weights with AVX2 integer kernels. */
/* Fused matvec: quantize x once, dot every row (optional F32 bias
   read from the blob at bias_off, -1 for none), multithreaded over
   rows. threads <= 1 runs single-threaded. */
void fang_llm_matvec(int64_t t, fang_str* blob, int64_t base_off, int64_t rows,
                     void* x, void* out, int64_t n, int64_t bias_off, int64_t threads);
void fang_llm_matvec0(int64_t t, fang_str* blob, int64_t base_off, int64_t rows,
                      void* x, void* out, int64_t n, int64_t threads);
/* Two and three weight matrices dotted against ONE shared activation
   in a single pool round-trip: one quantize, one dispatch. Bias
   offsets are blob-relative F32 rows, -1 for none. */
void fang_llm_matvec2(int64_t t1, int64_t t2, fang_str* blob,
                      int64_t o1, int64_t r1, int64_t o2, int64_t r2,
                      void* x, void* out1, void* out2, int64_t n, int64_t threads);
void fang_llm_matvec3(int64_t t1, int64_t t2, int64_t t3, fang_str* blob,
                      int64_t o1, int64_t r1, int64_t b1,
                      int64_t o2, int64_t r2, int64_t b2,
                      int64_t o3, int64_t r3, int64_t b3,
                      void* x, void* out1, void* out2, void* out3,
                      int64_t n, int64_t threads);
void fang_llm_rms_norm(void* x, void* w, void* out, int64_t n, double eps);
void fang_llm_rope(void* x, int64_t pos, int64_t heads, int64_t head_dim, double freq_base);
void fang_llm_kv_append(void* kcache, void* vcache, void* k, void* v, int64_t pos, int64_t kv_dim);
/* Causal attention over the KV cache, parallelized across heads. */
void fang_llm_attention(void* q, void* kcache, void* vcache, int64_t pos,
                        int64_t n_heads, int64_t n_kv_heads, int64_t head_dim,
                        void* out, int64_t threads);
/* One transformer layer: all parallel stages in one call with hot
   internal barriers (one condvar wake per token instead of per
   matvec). Norm weights are F32 rows of the blob. */
void fang_llm_layer(fang_str* blob,
                    int64_t hidden, int64_t kv_dim, int64_t ffn, int64_t pos,
                    int64_t n_heads, int64_t n_kv_heads, int64_t head_dim,
                    double freq_base, double eps, int64_t threads,
                    int64_t norm1_off, int64_t norm2_off,
                    int64_t tq, int64_t wq, int64_t bq,
                    int64_t tk, int64_t wk, int64_t bk,
                    int64_t tv, int64_t wv, int64_t bv,
                    int64_t to, int64_t wo,
                    int64_t tg, int64_t wg,
                    int64_t tu, int64_t wu,
                    int64_t td, int64_t wd,
                    void* h, void* xn, void* q, void* k, void* v,
                    void* kcache, void* vcache, void* attn_out,
                    void* gate, void* up, void* act, void* xn2);
/* End of a token's phase sequence. */
void fang_pool_seq_end(void);
void fang_llm_silu_mul(void* gate, void* up, void* out, int64_t n);
void fang_llm_add(void* a, void* b, void* out, int64_t n);
int64_t fang_llm_argmax(void* logits, int64_t n);
/* Build a string from a list of byte values (0-255). */
fang_str* fang_utf8_from_bytes(void* bytes);

/* Terminal raw mode (no echo, no line buffering, no signals). */
void fang_term_raw(void);
void fang_term_restore(void);
/* Read one key byte with a millisecond timeout; -1 when none. */
int64_t fang_read_key_ms(int64_t timeout_ms);
int64_t fang_tcp_send(void* sock, fang_str* data);
fang_str* fang_tcp_recv_all(void* sock);
void fang_tcp_close(void* sock);

/* ------------------------------------------------------------------ */
/* Closures: a function pointer plus an environment. Refcounted.      */
/* ------------------------------------------------------------------ */

typedef void (*fang_destroy_fn)(void* env);

typedef struct fang_closure {
    int64_t rc;
    void* env;
    void* fnptr;
    fang_destroy_fn destroy;
} fang_closure;

static inline fang_closure* fang_closure_new(void* env, void* fnptr, fang_destroy_fn destroy) {
    fang_closure* c = (fang_closure*)malloc(sizeof(fang_closure));
    if (!c) fang_panic("out of memory", "<runtime>", 0);
    c->rc = 1;
    c->env = env;
    c->fnptr = fnptr;
    c->destroy = destroy;
    return c;
}

static inline void fang_closure_decref(fang_closure* c) {
    if (c && c->rc != FANG_RC_STATIC && --c->rc == 0) {
        if (c->destroy && c->env) c->destroy(c->env);
        free(c);
    }
}

/* ------------------------------------------------------------------ */
/* Sorting for the stamped list types.                                 */
/* ------------------------------------------------------------------ */

void fang_sort_i64(int64_t* base, int64_t n);
void fang_sort_f64(double* base, int64_t n);
void fang_sort_str(fang_str** base, int64_t n);

/* ------------------------------------------------------------------ */
/* Collection templates, stamped by the codegen per instantiation.   */
/* SFX: type suffix. T: element type. EQ: equality expression.         */
/* INC / DEC: element refcount hooks, noop for primitives.            */
/* ------------------------------------------------------------------ */

#define FANG_LIST_DEFINE(SFX, T, EQ, INC, DEC)                                 \
  typedef struct fang_list_##SFX { int64_t rc, len, cap; T* elems; } fang_list_##SFX; \
  static fang_list_##SFX* fang_list_##SFX##_new(void) {                         \
    fang_list_##SFX* l = (fang_list_##SFX*)malloc(sizeof(*l));                  \
    l->rc = 1; l->len = 0; l->cap = 0; l->elems = NULL; return l;             \
  }                                                                           \
  static void fang_list_##SFX##_destroy(fang_list_##SFX* l) {                   \
    for (int64_t i = 0; i < l->len; i++) DEC(l->elems[i]);                    \
    free(l->elems); free(l);                                                   \
  }                                                                           \
  static inline void fang_list_##SFX##_decref(fang_list_##SFX* l) {             \
    if (l && l->rc != FANG_RC_STATIC && --l->rc == 0) fang_list_##SFX##_destroy(l); \
  }                                                                           \
  static void fang_list_##SFX##_push(fang_list_##SFX* l, T v) {                 \
    if (l->len == l->cap) {                                                   \
      l->cap = l->cap ? l->cap * 2 : 4;                                       \
      l->elems = (T*)realloc(l->elems, (size_t)l->cap * sizeof(T));           \
    }                                                                         \
    l->elems[l->len++] = v;                                                   \
  }                                                                           \
  static T fang_list_##SFX##_pop(fang_list_##SFX* l) {                          \
    if (l->len == 0) fang_panic("pop from an empty list", __FILE__, __LINE__); \
    return l->elems[--l->len];                                                \
  }                                                                           \
  static void fang_list_##SFX##_insert(fang_list_##SFX* l, int64_t i, T v) {    \
    if (i < 0) i += l->len;                                                   \
    if (i < 0 || i > l->len) fang_panic("insert index out of bounds", __FILE__, __LINE__); \
    fang_list_##SFX##_push(l, v);                                              \
    for (int64_t j = l->len - 1; j > i; j--) l->elems[j] = l->elems[j - 1];   \
    l->elems[i] = v;                                                          \
  }                                                                           \
  static void fang_list_##SFX##_remove_at(fang_list_##SFX* l, int64_t i) {      \
    if (i < 0) i += l->len;                                                   \
    if (i < 0 || i >= l->len) fang_panic("remove_at index out of bounds", __FILE__, __LINE__); \
    DEC(l->elems[i]);                                                         \
    for (int64_t j = i; j < l->len - 1; j++) l->elems[j] = l->elems[j + 1];   \
    l->len--;                                                                 \
  }                                                                           \
  static bool fang_list_##SFX##_contains(fang_list_##SFX* l, T v) {             \
    for (int64_t i = 0; i < l->len; i++) if (EQ(l->elems[i], v)) return true; \
    return false;                                                             \
  }                                                                           \
  static int64_t fang_list_##SFX##_index_of(fang_list_##SFX* l, T v) {          \
    for (int64_t i = 0; i < l->len; i++) if (EQ(l->elems[i], v)) return i;    \
    return -1;                                                                \
  }                                                                           \
  static void fang_list_##SFX##_reverse(fang_list_##SFX* l) {                   \
    for (int64_t i = 0, j = l->len - 1; i < j; i++, j--) {                    \
      T t = l->elems[i]; l->elems[i] = l->elems[j]; l->elems[j] = t;          \
    }                                                                         \
  }                                                                           \
  static T fang_list_##SFX##_get(fang_list_##SFX* l, int64_t i) {               \
    if (i < 0) i += l->len;                                                   \
    if (i < 0 || i >= l->len) fang_panic("list index out of bounds", __FILE__, __LINE__); \
    return l->elems[i];                                                       \
  }                                                                           \
  static void fang_list_##SFX##_set(fang_list_##SFX* l, int64_t i, T v) {       \
    if (i < 0) i += l->len;                                                   \
    if (i < 0 || i >= l->len) fang_panic("list index out of bounds", __FILE__, __LINE__); \
    DEC(l->elems[i]);                                                         \
    l->elems[i] = v;                                                          \
  }                                                                           \
  static fang_list_##SFX* fang_list_##SFX##_slice(fang_list_##SFX* l, int64_t a, int64_t b) { \
    if (a < 0) a += l->len;                                                   \
    if (b < 0) b += l->len;                                                   \
    if (a < 0) a = 0;                                                         \
    if (b > l->len) b = l->len;                                               \
    fang_list_##SFX* out = fang_list_##SFX##_new();                             \
    for (int64_t i = a; i < b; i++) { INC(l->elems[i]); fang_list_##SFX##_push(out, l->elems[i]); } \
    return out;                                                               \
  }                                                                           \
  static fang_list_##SFX* fang_list_##SFX##_concat(fang_list_##SFX* x, fang_list_##SFX* y) { \
    fang_list_##SFX* out = fang_list_##SFX##_new();                             \
    for (int64_t i = 0; i < x->len; i++) { INC(x->elems[i]); fang_list_##SFX##_push(out, x->elems[i]); } \
    for (int64_t i = 0; i < y->len; i++) { INC(y->elems[i]); fang_list_##SFX##_push(out, y->elems[i]); } \
    return out;                                                               \
  }

#define FANG_NOOP(x) ((void)0)
#define FANG_EQ_I64(a, b) ((a) == (b))
#define FANG_EQ_F64(a, b) ((a) == (b))
#define FANG_EQ_B8(a, b) ((a) == (b))
#define FANG_EQ_PTR(a, b) ((a) == (b))
#define FANG_EQ_STR(a, b) fang_str_eq((a), (b))
#define FANG_EQ_NEVER(a, b) (false)
#define FANG_INC_STR(x) fang_incref(x)
#define FANG_DEC_STR(x) fang_str_decref(x)

/* Tuples: plain value pairs. */
#define FANG_TUPLE_DEFINE(SFXA, SFXB, A, B)                                    \
  typedef struct fang_tuple_##SFXA##_##SFXB { A a; B b; } fang_tuple_##SFXA##_##SFXB;

/* Dicts: open addressing, insertion ordered via probe sequence scan.
   KHASH: hash expr, KEQ: equality expr. Requires the K and V list
   stamps and the tuple stamp to already exist in this translation
   unit; the codegen guarantees that ordering. */
#define FANG_DICT_DEFINE(KS, VS, K, V, KHASH, KEQ, DEC_K, DEC_V, KINC, VINC)   \
  typedef struct fang_dslot_##KS##_##VS { int64_t order; K key; V val; } fang_dslot_##KS##_##VS; \
  typedef struct fang_dict_##KS##_##VS { int64_t rc, len, cap; fang_dslot_##KS##_##VS* slots; } fang_dict_##KS##_##VS; \
  static fang_dict_##KS##_##VS* fang_dict_##KS##_##VS##_new(void) {             \
    fang_dict_##KS##_##VS* d = (fang_dict_##KS##_##VS*)malloc(sizeof(*d));     \
    d->rc = 1; d->len = 0; d->cap = 8;                                        \
    d->slots = (fang_dslot_##KS##_##VS*)calloc(8, sizeof(*d->slots));           \
    for (int64_t i = 0; i < 8; i++) d->slots[i].order = -1;                     \
    return d;                                                                 \
  }                                                                           \
  static void fang_dict_##KS##_##VS##_destroy(fang_dict_##KS##_##VS* d) {       \
    for (int64_t i = 0; i < d->cap; i++)                                      \
      if (d->slots[i].order >= 0) { DEC_K(d->slots[i].key); DEC_V(d->slots[i].val); } \
    free(d->slots); free(d);                                                   \
  }                                                                           \
  static inline void fang_dict_##KS##_##VS##_decref(fang_dict_##KS##_##VS* d) { \
    if (d && d->rc != FANG_RC_STATIC && --d->rc == 0) fang_dict_##KS##_##VS##_destroy(d); \
  }                                                                           \
  static void fang_dict_##KS##_##VS##_grow(fang_dict_##KS##_##VS* d) {          \
    int64_t ncap = d->cap * 2;                                                \
    fang_dslot_##KS##_##VS* ns = (fang_dslot_##KS##_##VS*)calloc((size_t)ncap, sizeof(*ns)); \
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
  static void fang_dict_##KS##_##VS##_reserve(fang_dict_##KS##_##VS* d, int64_t n) { \
    while (d->cap * 8 < (n + 1) * 10) fang_dict_##KS##_##VS##_grow(d);         \
  }                                                                           \
  static void fang_dict_##KS##_##VS##_set(fang_dict_##KS##_##VS* d, K k, V v) { \
    /* Tombstones count toward the load factor so a churned table grows */   \
    /* instead of filling its probe space.                                   */ \
    if ((d->len + 1) * 10 >= d->cap * 8) fang_dict_##KS##_##VS##_grow(d);      \
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
  static bool fang_dict_##KS##_##VS##_contains(fang_dict_##KS##_##VS* d, K k) { \
    uint64_t h = (uint64_t)(KHASH(k));                                        \
    int64_t j = (int64_t)(h & (uint64_t)(d->cap - 1));                        \
    while (d->slots[j].order != -1) {                                          \
      if (d->slots[j].order != -2 && KEQ(d->slots[j].key, k)) return true;     \
      j = (j + 1) & (d->cap - 1);                                             \
    }                                                                         \
    return false;                                                             \
  }                                                                           \
  static V fang_dict_##KS##_##VS##_get(fang_dict_##KS##_##VS* d, K k, V def) {  \
    uint64_t h = (uint64_t)(KHASH(k));                                        \
    int64_t j = (int64_t)(h & (uint64_t)(d->cap - 1));                        \
    while (d->slots[j].order != -1) {                                          \
      if (d->slots[j].order != -2 && KEQ(d->slots[j].key, k)) return d->slots[j].val; \
      j = (j + 1) & (d->cap - 1);                                             \
    }                                                                         \
    return def;                                                               \
  }                                                                           \
  static bool fang_dict_##KS##_##VS##_remove(fang_dict_##KS##_##VS* d, K k) {   \
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
  static fang_list_##KS* fang_dict_##KS##_##VS##_keys(fang_dict_##KS##_##VS* d) { \
    fang_list_##KS* out = fang_list_##KS##_new();                               \
    for (int64_t ord = 0; ord < d->len; ord++)                                 \
      for (int64_t i = 0; i < d->cap; i++)                                     \
        if (d->slots[i].order == ord) { KINC(d->slots[i].key); fang_list_##KS##_push(out, d->slots[i].key); } \
    return out;                                                               \
  }                                                                           \
  static fang_list_##VS* fang_dict_##KS##_##VS##_values(fang_dict_##KS##_##VS* d) { \
    fang_list_##VS* out = fang_list_##VS##_new();                                \
    for (int64_t ord = 0; ord < d->len; ord++)                                 \
      for (int64_t i = 0; i < d->cap; i++)                                     \
        if (d->slots[i].order == ord) { VINC(d->slots[i].val); fang_list_##VS##_push(out, d->slots[i].val); } \
    return out;                                                               \
  }                                                                           \
  static fang_list_t_##KS##_##VS* fang_dict_##KS##_##VS##_items(fang_dict_##KS##_##VS* d) { \
    fang_list_t_##KS##_##VS* out = fang_list_t_##KS##_##VS##_new();              \
    for (int64_t ord = 0; ord < d->len; ord++)                                 \
      for (int64_t i = 0; i < d->cap; i++)                                     \
        if (d->slots[i].order == ord) {                                         \
          fang_tuple_##KS##_##VS t = { d->slots[i].key, d->slots[i].val };      \
          KINC(t.a); VINC(t.b);                                                \
          fang_list_t_##KS##_##VS##_push(out, t);                               \
        }                                                                     \
    return out;                                                               \
  }

#define FANG_HASH_I64(k) ((uint64_t)((int64_t)(k) * 0x9E3779B97F4A7C15ULL))
#define FANG_HASH_STR(k) fang_str_hash((k))

#endif /* FANG_RT_H */
