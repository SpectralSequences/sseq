from typing import TypeVar, Generic, Dict, Any, Callable, Optional

T = TypeVar("T")
class SignalDict(dict, Generic[T]):
    """ This is a dict that signals when it is changed, by calling self._parent._needs_update()
        if self._parent is defined or by calling self._callback() if self._callback is defined,
        or both.
    """
    def __init__(self, 
        d : Dict[str, T] = {}, 
        *, 
        parent : Any = None, 
        callback : Optional[Callable[[], None]] = None,
        **kwargs : Any
    ):
        dict.__init__(self, d, **kwargs)
        self._set_parent(parent)
        self._callback = callback
    
    def _set_parent(self, parent):
        self._parent = parent

    def _needs_update(self):
        if self._parent:
            self._parent._needs_update()
        if self._callback:
            self._callback()

    def __setitem__(self, key : str, val : T):
        if hasattr(val, "set_parent"):
            val.set_parent(self)
        self._needs_update()
        return dict.__setitem__(self, key, val)

    def __getitem__(self, key : str) -> T:
        return dict.__getitem__(self, key)