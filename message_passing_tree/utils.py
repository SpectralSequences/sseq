import json

def stringifier(obj):
    if hasattr(obj, "to_json"):
        return obj.to_json()
    elif hasattr(obj, "__dict__"):
        return obj.__dict__
    else:
        return str(obj)

def json_stringify(obj):
    return json.dumps(obj, default=stringifier)

def arguments(*args, **kwargs):
    return [args, kwargs]