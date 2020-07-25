#define PY_SSIZE_T_CLEAN
#include <Python.h>
#include "frameobject.h"


static char module_docstring[] =
    "This module uses tracing to execute a callback once every fixed number of lines of python code.\n"
    "The purpose of this package is to implement very very rudimentary multitasking on top of Pyodide.\n"
    "Emscripten / Webworkers only support cooperative multitasking. Python expects to run inside of an\n"
    "operating system with preemptive multitasking, and various basic features like KeyboardInterrupt\n"
    "cannot function without it. This package is part of a hacky fix for that.";

static char start_docstring[] =
    "Start inspection.\n"
    "\n"
    "Argument:\n"
    "callback -- Will call this function every 'interval' lines of python execution.";

static char end_docstring[] =
    "End inspection.";

static char get_interval_docstring[] =
    "Get the 'time interval' at which the callback gets called.";

static char set_interval_docstring[] =
    "Set the 'time interval' at which the callback gets called.\n"
    "\n"
    "Argument:\n"
    "interval -- how many lines to wait between calls to callback.";


static long trace_inspect_interval = -1;
static long tracetick = -1;

static PyObject *crappy_multitasking__start(PyObject *self,  PyObject *args);
static PyObject *crappy_multitasking__end(PyObject *self,  PyObject *args);
static PyObject *crappy_multitasking__get_interval(PyObject *self,  PyObject *args);
static PyObject *crappy_multitasking__set_interval(PyObject *self,  PyObject *args);
static int
trace_trampoline(PyObject *self, PyFrameObject *frame,
                 int what, PyObject *arg);
static int
callback_trampoline(PyObject* callback);


static PyMethodDef module_methods[] = {
    {"start", crappy_multitasking__start, METH_VARARGS, start_docstring},
    {"end", crappy_multitasking__end, METH_VARARGS, end_docstring},
    {"get_interval", crappy_multitasking__get_interval, METH_VARARGS, get_interval_docstring},
    {"set_interval", crappy_multitasking__set_interval, METH_VARARGS, set_interval_docstring},
    {NULL, NULL, 0, NULL}
};

static struct PyModuleDef module = {
    PyModuleDef_HEAD_INIT,
    "crappy_multitasking",   /* name of module */
    module_docstring, /* module documentation, may be NULL */
    -1,       /* size of per-interpreter state of the module,
                 or -1 if the module keeps state in global variables. */
    module_methods
};


PyMODINIT_FUNC PyInit_crappy_multitasking(void)
{
    return PyModule_Create(&module);
}









static PyObject *crappy_multitasking__start(PyObject *self,  PyObject *args){
    if(trace_inspect_interval <= 0){
        PyErr_SetString(PyExc_ValueError, "You must call 'set_interval' before calling 'start'.");
        return NULL;
    }
    tracetick = trace_inspect_interval;
    PyObject *callback;
    if(!PyArg_ParseTuple(args, "O", &callback)){
        return NULL;
    }
    PyEval_SetTrace(trace_trampoline, callback);
    Py_RETURN_NONE;
}

static PyObject *crappy_multitasking__end(PyObject *self,  PyObject *args){
    PyEval_SetTrace(NULL, NULL);
    Py_RETURN_NONE;
}

static PyObject *crappy_multitasking__get_interval(PyObject *self,  PyObject *args){
    return PyLong_FromLong(trace_inspect_interval);
}

static PyObject *crappy_multitasking__set_interval(PyObject *self,  PyObject *args){
    long new_interval;
    if(!PyArg_ParseTuple(args, "l", &new_interval)){
        return NULL;
    }
    if(new_interval <= 0){
        PyErr_SetString(PyExc_ValueError, "Interval should be nonnegative!");
        return NULL;
    }
    trace_inspect_interval = new_interval;
    Py_RETURN_NONE;
}

static int
trace_trampoline(PyObject *self, PyFrameObject *frame,
                 int what, PyObject *arg)
{
    PyObject *callback = self;
    tracetick --;
    if(tracetick == 0){
        tracetick = trace_inspect_interval;
        return callback_trampoline(callback);
    }
    return 0;
}


static int
callback_trampoline(PyObject* callback)
{
    PyObject* result = _PyObject_CallNoArg(callback);
    if (result == NULL) {
        PyEval_SetTrace(NULL, NULL);
        return -1;
    }
    Py_DECREF(result);
    return 0;
}