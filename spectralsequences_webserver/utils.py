import json
import pathlib
import sys 

def exec_file_if_exists(path, globals, locals):
    if path.is_file():
        code = compile(path.read_text(), path, "exec")
        exec(code, globals, locals) 

def exec_file(path, globals, locals):
    code = compile(path.read_text(), path, "exec")
    exec(code, globals, locals)

def append_key_to_json_str(json_str, key, value):
    return json_string[:-1] + f""", "{str(key)}": {str(value)}}}"""

def stringifier(obj):
    if hasattr(obj, "to_json"):
        return obj.to_json()
    else:
        return obj.__dict__
        

def json_stringify(obj):
    return json.dumps(obj, default=stringifier)

def bind(instance, func, as_name=None):
    """
    Bind the function *func* to *instance*, with either provided name *as_name*
    or the existing name of *func*. The provided *func* should accept the 
    instance as the first argument, i.e. "self".
    """
    if as_name is None:
        as_name = func.__name__ 
    bound_method = func.__get__(instance, instance.__class__)
    setattr(instance, as_name, bound_method)
    return bound_method

def public_fields(obj):
    result = {}
    for field in dir(obj):
        if not field.startswith("_") and not callable(getattr(obj,field)):
            result[field] = getattr(obj, field)
    return result

def assign_fields(obj, kwargs, fields):
    for field in fields: 
        if field["type"] == "mandatory":
            assign_kwarg_mandatory(obj, kwargs, field["field"])
        elif field["type"] == "optional":
            assign_kwarg_optional(obj, kwargs, field["field"])
        elif field["type"] == "default":
            assign_kwarg_default(obj, kwargs, field["field"], field["default"])
        else:
            field_type = field["type"]
            raise ValueError(f"Unknown field type {field_type}")

def assign_kwarg_mandatory(obj, kwargs, field):
    if field in kwargs:
        setattr(obj,field, kwargs[field])
    else:
        raise KeyError(f"Missing mandatory argument {field}");


def assign_kwarg_optional(obj, kwargs, field):
    if field in kwargs:
        setattr(obj,field, kwargs[field])
    else:
        pass

def assign_kwarg_default(obj, kwargs, field, default_value):
    if field in kwargs:
        setattr(obj, field, kwargs[field]) 
    else:
        setattr(obj, field, default_value) 
