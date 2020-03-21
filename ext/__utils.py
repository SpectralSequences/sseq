import sys
import rust_ext
def export_all_rust_names(module_path):
    module = sys.modules[module_path]
    rust_module = get_rust_module(module_path)
    for name in getattr(rust_module, "__all__"):
        setattr(module, name, getattr(rust_module, name))

def get_rust_module(module_path):
    split_path = module_path.split(".")[1:]
    split_path[0] = remove_prefix_if_present(split_path[0], "rust_")
    rust_module_name = ".".join(split_path)
    return getattr(rust_ext, rust_module_name)

def remove_prefix_if_present(s, prefix):
    if s.startswith(prefix):
        return s[len(prefix):]
    else:
        return s

