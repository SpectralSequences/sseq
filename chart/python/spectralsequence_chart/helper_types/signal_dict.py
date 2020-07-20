from typing import TypeVar, Generic, Dict, Any

T = TypeVar("T")
class SignalDict(dict, Generic[T]):
    def __init__(self, d : Dict[str, T] = {}, *, parent : Any = None, **kwargs : Any):
        dict.__init__(self, d, **kwargs)
        self.set_parent(parent)
    
    def set_parent(self, parent):
        self.parent = parent

    def needs_update(self):
        if self.parent:
            self.parent.needs_update()

    def __setitem__(self, key : str, val : T):
        if hasattr(val, "set_parent"):
            val.set_parent(self)
        self.needs_update()
        return dict.__setitem__(self, key, val)

    def __getitem__(self, key : str) -> T:
        return dict.__getitem__(self, key)