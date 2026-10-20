/* Embedded CPython bridge implementation. Requires Python dev headers;
   linked only for --python builds. */

#include "fang_py.h"
#include <Python.h>

void fang_py_init(void) {
    if (!Py_IsInitialized()) {
        Py_Initialize();
    }
}

void* fang_py_import(const char* name) {
    PyObject* m = PyImport_ImportModule(name);
    if (!m) {
        PyErr_Print();
        fang_panic("py.import failed", __FILE__, __LINE__);
    }
    return m;
}

void* fang_py_attr(void* obj, const char* name) {
    PyObject* v = PyObject_GetAttrString((PyObject*)obj, name);
    if (!v) {
        PyErr_Print();
        fang_panic("py.attr failed", __FILE__, __LINE__);
    }
    return v;
}

void* fang_py_call(void* fn, void* const* args, long n) {
    PyObject* tuple = PyTuple_New(n);
    for (long i = 0; i < n; i++) {
        Py_INCREF((PyObject*)args[i]);
        PyTuple_SET_ITEM(tuple, i, (PyObject*)args[i]);
    }
    PyObject* r = PyObject_CallObject((PyObject*)fn, tuple);
    Py_DECREF(tuple);
    if (!r) {
        PyErr_Print();
        fang_panic("py.call failed", __FILE__, __LINE__);
    }
    return r;
}

void* fang_py_call_method(void* obj, const char* name, void* const* args, long n) {
    PyObject* m = PyObject_GetAttrString((PyObject*)obj, name);
    if (!m) {
        PyErr_Print();
        fang_panic("py.call_method: no such method", __FILE__, __LINE__);
    }
    PyObject* tuple = PyTuple_New(n);
    for (long i = 0; i < n; i++) {
        Py_INCREF((PyObject*)args[i]);
        PyTuple_SET_ITEM(tuple, i, (PyObject*)args[i]);
    }
    PyObject* r = PyObject_CallObject(m, tuple);
    Py_DECREF(m);
    Py_DECREF(tuple);
    if (!r) {
        PyErr_Print();
        fang_panic("py.call_method failed", __FILE__, __LINE__);
    }
    return r;
}

void* fang_py_eval(const char* code) {
    PyObject* r = PyRun_String(code, Py_eval_input, PyEval_GetBuiltins(), PyEval_GetBuiltins());
    if (!r) {
        PyErr_Print();
        fang_panic("py.eval failed", __FILE__, __LINE__);
    }
    return r;
}

void fang_py_exec(const char* code) {
    PyObject* r = PyRun_String(code, Py_file_input, PyEval_GetBuiltins(), PyEval_GetBuiltins());
    if (!r) {
        PyErr_Print();
        fang_panic("py.exec failed", __FILE__, __LINE__);
    }
    Py_DECREF(r);
}

/* Fang list layout: {rc, len, cap, elems} — read elems/len directly. */
typedef struct {
    int64_t rc, len, cap;
    void** elems;
} fang_py_list_shim;

void* fang_py_call_list(void* fn, void* fang_list_ptr) {
    fang_py_list_shim* l = (fang_py_list_shim*)fang_list_ptr;
    return fang_py_call(fn, l ? l->elems : NULL, l ? (long)l->len : 0);
}

void* fang_py_call_list_method(void* obj, const char* name, void* fang_list_ptr) {
    fang_py_list_shim* l = (fang_py_list_shim*)fang_list_ptr;
    return fang_py_call_method(obj, name, l ? l->elems : NULL, l ? (long)l->len : 0);
}

void* fang_py_from_int(int64_t v) { return PyLong_FromLongLong(v); }
void* fang_py_from_float(double v) { return PyFloat_FromDouble(v); }
void* fang_py_from_bool(int v) { return PyBool_FromLong(v); }

void* fang_py_from_str(const char* s, long len) {
    return PyUnicode_DecodeUTF8(s, len, "strict");
}

int64_t fang_py_to_int(void* o) { return PyLong_AsLongLong((PyObject*)o); }
double fang_py_to_float(void* o) { return PyFloat_AsDouble((PyObject*)o); }
int fang_py_to_bool(void* o) { return PyObject_IsTrue((PyObject*)o); }

fang_str* fang_py_to_str(void* o) {
    Py_ssize_t len = 0;
    const char* s = PyUnicode_AsUTF8AndSize((PyObject*)o, &len);
    if (!s) {
        PyErr_Print();
        fang_panic("py.to_str: not a str", __FILE__, __LINE__);
    }
    return fang_str_new(s, (int64_t)len);
}

fang_str* fang_py_repr(void* o) {
    PyObject* r = PyObject_Repr((PyObject*)o);
    if (!r) {
        PyErr_Print();
        fang_panic("py.repr failed", __FILE__, __LINE__);
    }
    fang_str* s = fang_py_to_str(r);
    Py_DECREF(r);
    return s;
}

void* fang_py_none(void) {
    Py_INCREF(Py_None);
    return Py_None;
}

int fang_py_is_none(void* o) { return o == Py_None; }

void fang_py_decref(void* o) { Py_XDECREF((PyObject*)o); }

void fang_py_print(void* o) {
    PyObject* r = PyObject_Repr((PyObject*)o);
    if (!r) {
        PyErr_Print();
        fang_panic("py.print failed", __FILE__, __LINE__);
    }
    const char* s = PyUnicode_AsUTF8(r);
    if (s) fputs(s, stdout);
    Py_DECREF(r);
}
