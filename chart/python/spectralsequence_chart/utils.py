import json

def stringifier(obj):
    if hasattr(obj, "to_json"):
        return obj.to_json()
    elif hasattr(obj, "__dict__"):
        return obj.__dict__
    else:
        return str(obj)


class JSON:
    @staticmethod
    def stringify(obj):
        return json.dumps(obj, default=stringifier)

    @staticmethod
    def parse(json_str):
        return json.loads(json_str, object_hook = JSON.parser_object_hook )

    def parser_object_hook(json_dict):
        JSON.ensure_types_are_initialized()
        if "type" not in json_dict:
            return json_dict
        return JSON.types[json_dict["type"]].from_json(json_dict)

    def ensure_types_are_initialized():
        if hasattr(JSON, "types"):
            return
        from .chart_elements import (
            ChartClass,
            ChartStructline, 
            ChartDifferential, 
            ChartExtension
        )
        from .chart import SseqChart
        from .page_property import PageProperty
        JSON.types = { t.__name__ : t for t in [
            SseqChart,
            ChartClass,
            ChartStructline, 
            ChartDifferential, 
            ChartExtension,
            PageProperty
        ]}

PROPERTY_PREFIX = "_property_"


def public_keys(obj):
    return [field for field in dir(obj) \
            if not field.startswith("_") and not callable(getattr(obj,field))]

def public_fields(obj):
    result = {}
    for field in public_keys(obj):
        value = getattr(obj, field)
        if value is not None:
            result[field] = value
    for field in (field for field in dir(obj) if field.startswith(PROPERTY_PREFIX)):
        value = getattr(obj, field)
        if value is not None:
            result[field[len(PROPERTY_PREFIX):]] = value
    return result




class MyProperty:
    "Emulate PyProperty_Type() in Objects/descrobject.c"

    def __init__(self, fget=None, fset=None, fdel=None, doc=None):
        self.fget = fget
        self.fset = fset
        self.fdel = fdel
        if fget is not None:
            self.name = fget.__name__
            self.storage_name = PROPERTY_PREFIX + self.name

        if doc is None and fget is not None:
            doc = fget.__doc__
        self.__doc__ = doc

    def __get__(self, obj, objtype=None):
        if obj is None:
            return self
        if self.fget is None:
            raise AttributeError("unreadable attribute")
        return self.fget(obj, self.storage_name)

    def __set__(self, obj, value):
        if self.fset is None:
            print("Can't set:", obj, value)
            raise AttributeError("can't set attribute")
        self.fset(obj, self.storage_name, value)

    def __delete__(self, obj):
        if self.fdel is None:
            raise AttributeError("can't delete attribute")
        self.fdel(obj)

    def getter(self, fget):
        return type(self)(fget, self.fset, self.fdel, self.__doc__)

    def setter(self, fset):
        return type(self)(self.fget, fset, self.fdel, self.__doc__)

    def deleter(self, fdel):
        return type(self)(self.fget, self.fset, fdel, self.__doc__)

def my_property(fget):
    return MyProperty().getter(fget)

def sseq_property(func):
    def getter(self, storage_name):
        return getattr(self, storage_name, None)
    getter.__name__ = func.__name__
    getter = my_property(getter)

    @getter.setter
    def setter(self, storage_name, value):
        setattr(self, storage_name, value)
        func(self, storage_name)
    return setter



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

def copy_fields_from_kwargs(obj, kwargs):
    for [k, v] in kwargs.items():
        try:
            setattr(obj, k, v)
        except AttributeError:
            print("can't set:", k, v)

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


def replace_keys(d, replace_keys):
    for [key, replacement] in replace_keys:
        if hasattr(d, key):
            setattr(d, replacement, getattr(d, key))
            delattr(d, key)

def reverse_replace_keys(d, replace_keys):
    for [replacement, key] in replace_keys:
        if hasattr(d, key):
            setattr(d, replacement, getattr(d, key))
            delattr(d, key)

def arguments(*args, **kwargs):
    return [args, kwargs]