from typing import TypeVar, Generic, Dict, Any, Callable, Optional, List, Iterable
from collections.abc import MutableMapping, MutableSequence


T = TypeVar("T")
class SignalDict(MutableMapping, Generic[T]):
    """ This is a dict that signals when it is changed, by calling self._parent._needs_update()
        if self._parent is defined or by calling self._callback() if self._callback is defined,
        or both.
    """
    def __init__(self, 
        d : Dict[str, T] = None, 
        *, 
        parent : Any = None, 
        callback : Optional[Callable[[], None]] = None,
    ):
        self._dict = d or {}
        self.set_parent(parent)
        self._callback = callback
    
    def set_parent(self, parent):
        self._parent = parent

    def set_callback(self, callback):
        self._callback = callback

    def needs_update(self):
        if self._parent:
            self._parent._needs_update()
        if self._callback:
            self._callback()

    def to_json(self):
        result = dict(type=type(self).__name__)
        result.update(self._dict)
        return result

    @classmethod
    def from_json(cls, json):
        assert json.pop("type") == cls.__name__
        result = SignalDict(json)
        for v in result.values():
            if hasattr(v, "set_parent"):
                v.set_parent(result)
        return result

    def __setitem__(self, key : str, val : T):
        if hasattr(val, "set_parent"):
            val.set_parent(self)
        self._needs_update()
        self._dict[key] = val

    def __getitem__(self, key : str) -> T:
        return self._dict[key]
    
    def __delitem__(self, key : str):
        self._needs_update()
        del self._dict[key]

    def __iter__(self):
        return self._dict.__iter__()

    def __len__(self):
        return len(self._dict)

    def __repr__(self):
        return repr(self._dict)

class SignalList(MutableSequence, Generic[T]):
    """ This is a list that signals when it is changed, by calling self._parent._needs_update()
        if self._parent is defined or by calling self._callback() if self._callback is defined,
        or both.
    """
    def __init__(self, 
        l : List[T] = None,
        *, 
        parent : Any = None, 
        callback : Optional[Callable[[], None]] = None,
    ):
        self._list = l or []
        self.set_parent(parent)
        self._callback = callback
    
    def set_parent(self, parent):
        self._parent = parent

    def set_callback(self, callback):
        self._callback = callback

    def to_json(self):
        return dict(type=type(self).__name__, list = self._list)
        
    @classmethod
    def from_json(cls, json):
        assert json.pop("type") == cls.__name__
        result = SignalList(json["list"])
        for value in result:
            if hasattr(value, "set_parent"):
                value.set_parent(result)
        return result

    def _needs_update(self):
        if self._parent:
            self._parent._needs_update()
        if self._callback:
            self._callback()

    def __setitem__(self, key: slice, vals: Iterable[T]) -> None:
        self._needs_update()
        self._list[key] = vals
        for val in self._list[key]:
            if hasattr(val, "set_parent"):
                val.set_parent(self)        
        return 

    def __getitem__(self, key : slice) -> T:
        return self._list[key]

    def __delitem__(self, i: slice):
        self._needs_update()
        del self._list[i]
    
    def __len__(self) -> int:
        return len(self._list)
    
    def insert(self, index: int, value: T) -> None:
        self._needs_update()
        self._list.insert(index, value)

    def __repr__(self):
        return repr(self._list)