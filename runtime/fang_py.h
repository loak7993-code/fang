#ifndef FANG_PY_H
#define FANG_PY_H

#include "fang_rt.h"

/* Embedded CPython bridge. Compiled and linked only when the program
   is built with `fang build --python` (or `fang run x.fang --python`). */

void fang_py_init(void);

/* Import a module by name; returns a new reference (PyObject*). */
void* fang_py_import(const char* name);
/* Attribute fetch obj.name; new reference. */
void* fang_py_attr(void* obj, const char* name);
/* Call callable with a list of PyObject* arguments; new reference. */
void* fang_py_call(void* fn, void* const* args, long n);
/* Call with a Fang list[ptr] of arguments. */
void* fang_py_call_list(void* fn, void* fang_list_ptr);
/* Call obj.name with a Fang list[ptr] of arguments. */
void* fang_py_call_list_method(void* obj, const char* name, void* fang_list_ptr);
/* Call obj.name(...) with the arguments. */
void* fang_py_call_method(void* obj, const char* name, void* const* args, long n);
/* Evaluate a Python expression in the builtins namespace. */
void* fang_py_eval(const char* code);
/* Run a statement; returns nil on success, raises on error. */
void fang_py_exec(const char* code);

/* Conversions */
void* fang_py_from_int(int64_t v);
void* fang_py_from_float(double v);
void* fang_py_from_bool(int v);
void* fang_py_from_str(const char* s, long len);
int64_t fang_py_to_int(void* o);
double fang_py_to_float(void* o);
int fang_py_to_bool(void* o);
fang_str* fang_py_to_str(void* o);
fang_str* fang_py_repr(void* o);

/* None and membership */
void* fang_py_none(void);
int fang_py_is_none(void* o);
/* Manual reference drop; objects otherwise live for the process. */
void fang_py_decref(void* o);
/* Print to stdout via Python's repr. */
void fang_py_print(void* o);

#endif /* FANG_PY_H */
