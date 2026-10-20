/* Embedded CPython bridge implementation. Requires Python dev headers;
   linked only for --python builds. */

#include "leo_py.h"
#include <Python.h>

void leo_py_init(void) {
    if (!Py_IsInitialized()) {
        Py_Initialize();
    }
}

void* leo_py_import(const char* name) {
    PyObject* m = PyImport_ImportModule(name);
    if (!m) {
        PyErr_Print();
        leo_panic("py.import failed", __FILE__, __LINE__);
    }
    return m;
}

void* leo_py_attr(void* obj, const char* name) {
    PyObject* v = PyObject_GetAttrString((PyObject*)obj, name);
    if (!v) {
        PyErr_Print();
        leo_panic("py.attr failed", __FILE__, __LINE__);
    }
    return v;
}

void* leo_py_call(void* fn, void* const* args, long n) {
    PyObject* tuple = PyTuple_New(n);
    for (long i = 0; i < n; i++) {
        Py_INCREF((PyObject*)args[i]);
        PyTuple_SET_ITEM(tuple, i, (PyObject*)args[i]);
    }
    PyObject* r = PyObject_CallObject((PyObject*)fn, tuple);
    Py_DECREF(tuple);
    if (!r) {
        PyErr_Print();
        leo_panic("py.call failed", __FILE__, __LINE__);
    }
    return r;
}

void* leo_py_call_method(void* obj, const char* name, void* const* args, long n) {
    PyObject* m = PyObject_GetAttrString((PyObject*)obj, name);
    if (!m) {
        PyErr_Print();
        leo_panic("py.call_method: no such method", __FILE__, __LINE__);
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
        leo_panic("py.call_method failed", __FILE__, __LINE__);
    }
    return r;
}

void* leo_py_eval(const char* code) {
    PyObject* r = PyRun_String(code, Py_eval_input, PyEval_GetBuiltins(), PyEval_GetBuiltins());
    if (!r) {
        PyErr_Print();
        leo_panic("py.eval failed", __FILE__, __LINE__);
    }
    return r;
}

void leo_py_exec(const char* code) {
    PyObject* r = PyRun_String(code, Py_file_input, PyEval_GetBuiltins(), PyEval_GetBuiltins());
    if (!r) {
        PyErr_Print();
        leo_panic("py.exec failed", __FILE__, __LINE__);
    }
    Py_DECREF(r);
}

/* Leo list layout: {rc, len, cap, elems} — read elems/len directly. */
typedef struct {
    int64_t rc, len, cap;
    void** elems;
} leo_py_list_shim;

void* leo_py_call_list(void* fn, void* leo_list_ptr) {
    leo_py_list_shim* l = (leo_py_list_shim*)leo_list_ptr;
    return leo_py_call(fn, l ? l->elems : NULL, l ? (long)l->len : 0);
}

void* leo_py_call_list_method(void* obj, const char* name, void* leo_list_ptr) {
    leo_py_list_shim* l = (leo_py_list_shim*)leo_list_ptr;
    return leo_py_call_method(obj, name, l ? l->elems : NULL, l ? (long)l->len : 0);
}

void* leo_py_from_int(int64_t v) { return PyLong_FromLongLong(v); }
void* leo_py_from_float(double v) { return PyFloat_FromDouble(v); }
void* leo_py_from_bool(int v) { return PyBool_FromLong(v); }

void* leo_py_from_str(const char* s, long len) {
    return PyUnicode_DecodeUTF8(s, len, "strict");
}

int64_t leo_py_to_int(void* o) { return PyLong_AsLongLong((PyObject*)o); }
double leo_py_to_float(void* o) { return PyFloat_AsDouble((PyObject*)o); }
int leo_py_to_bool(void* o) { return PyObject_IsTrue((PyObject*)o); }

leo_str* leo_py_to_str(void* o) {
    Py_ssize_t len = 0;
    const char* s = PyUnicode_AsUTF8AndSize((PyObject*)o, &len);
    if (!s) {
        PyErr_Print();
        leo_panic("py.to_str: not a str", __FILE__, __LINE__);
    }
    return leo_str_new(s, (int64_t)len);
}

leo_str* leo_py_repr(void* o) {
    PyObject* r = PyObject_Repr((PyObject*)o);
    if (!r) {
        PyErr_Print();
        leo_panic("py.repr failed", __FILE__, __LINE__);
    }
    leo_str* s = leo_py_to_str(r);
    Py_DECREF(r);
    return s;
}

void* leo_py_none(void) {
    Py_INCREF(Py_None);
    return Py_None;
}

int leo_py_is_none(void* o) { return o == Py_None; }

void leo_py_decref(void* o) { Py_XDECREF((PyObject*)o); }

void leo_py_print(void* o) {
    PyObject* r = PyObject_Repr((PyObject*)o);
    if (!r) {
        PyErr_Print();
        leo_panic("py.print failed", __FILE__, __LINE__);
    }
    const char* s = PyUnicode_AsUTF8(r);
    if (s) fputs(s, stdout);
    Py_DECREF(r);
}
