#ifndef LEO_PY_H
#define LEO_PY_H

#include "leo_rt.h"

/* Embedded CPython bridge. Compiled and linked only when the program
   is built with `leo build --python` (or `leo run x.leo --python`). */

void leo_py_init(void);

/* Import a module by name; returns a new reference (PyObject*). */
void* leo_py_import(const char* name);
/* Attribute fetch obj.name; new reference. */
void* leo_py_attr(void* obj, const char* name);
/* Call callable with a list of PyObject* arguments; new reference. */
void* leo_py_call(void* fn, void* const* args, long n);
/* Call with a Leo list[ptr] of arguments. */
void* leo_py_call_list(void* fn, void* leo_list_ptr);
/* Call obj.name with a Leo list[ptr] of arguments. */
void* leo_py_call_list_method(void* obj, const char* name, void* leo_list_ptr);
/* Call obj.name(...) with the arguments. */
void* leo_py_call_method(void* obj, const char* name, void* const* args, long n);
/* Evaluate a Python expression in the builtins namespace. */
void* leo_py_eval(const char* code);
/* Run a statement; returns nil on success, raises on error. */
void leo_py_exec(const char* code);

/* Conversions */
void* leo_py_from_int(int64_t v);
void* leo_py_from_float(double v);
void* leo_py_from_bool(int v);
void* leo_py_from_str(const char* s, long len);
int64_t leo_py_to_int(void* o);
double leo_py_to_float(void* o);
int leo_py_to_bool(void* o);
leo_str* leo_py_to_str(void* o);
leo_str* leo_py_repr(void* o);

/* None and membership */
void* leo_py_none(void);
int leo_py_is_none(void* o);
/* Manual reference drop; objects otherwise live for the process. */
void leo_py_decref(void* o);
/* Print to stdout via Python's repr. */
void leo_py_print(void* o);

#endif /* LEO_PY_H */
