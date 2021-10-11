from .. import config
from .. import utils

namespace_functions = []
default_namespace = []
namespace_initialized = False


def namespace(namespace_list):
    namespace_functions.append(namespace_list)

def get_default_namespace():
    if namespace_initialized:
        return default_namespace
    for namespace_list in namespace_functions:
        default_namespace.extend(namespace_list())
    return default_namespace

def add_to_namespace(namespace, obj):
    if hasattr(obj, "__name__"):
        name = obj.__name__.split(".")[-1]
    else:
        [obj, name] = obj
    namespace[name] = obj

def add_stuff_to_namespace(namespace, to_add):
    for name in to_add:
        add_to_namespace(namespace, name)

utils.exec_file(config.NAMESPACE_FILE, globals(), locals())
utils.exec_file_if_exists(config.USER_NAMESPACE_FILE, globals(), locals())