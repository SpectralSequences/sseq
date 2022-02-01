from .infinity import INFINITY
import json
from typing import List, Tuple, Any, Type, Union, TypeVar, Generic, Optional, Dict, cast, Callable

T = TypeVar('T')
class PageProperty(Generic[T]):
    """
        A class to represent a property that varies depending on the pages of a spectral sequence. 
        This is the main helper class that encapsulates any property of a class, edge, or chart
        that varies depending on the page.

        Examples:

            >>> p = PageProperty(1)
            >>> p[4] = 7
            >>> p[2]
            1
            >>> p[4]
            7
    """
    def __init__(self, 
        value : T, 
        parent : Optional[Any] = None,
        callback : Optional[Callable[[], None]] = None,
    ):
        """ Initialize the PageProperty to always have value v."""
        self._values : List[Tuple[int, T]] = [(0, value)]
        self.set_parent(parent)
        self._callback = callback

    def set_parent(self, parent : Optional[Any]):
        self._parent = parent
    

    def set_callback(self, callback : Callable[[], None]):
        self._callback = callback

    def _needs_update(self):
        if self._parent:
            self._parent._needs_update()
        if self._callback:
            self._callback()

    def _find_index(self, target_page : int) -> Tuple[int, bool]:
        result_idx = None
        for (idx, (page, _)) in enumerate(self._values):
            if page > target_page:
                break
            result_idx = idx 
        # We need to help out the type checker here
        if result_idx is None: 
            raise ValueError(f"Page Property indexed with negative index: {target_page}")
        return (result_idx, self._values[result_idx][0] == target_page)

    def __getitem__(self, x : Union[int, slice]) -> T:
        stop = None
        if type(x) == slice:
            stop = x.stop or INFINITY
            x = x.start or 0

        if type(x) != int:
            raise TypeError(f"Expected integer, got {type(x).__name__}.")

        assert type(x) is int # Make type analysis thing happy
        (idx, _) = self._find_index(x)
        if stop:
            (idx2, _) = self._find_index(stop - 1)
            if idx != idx2:
                raise ValueError("Indexed with slice but value is inconsistent across slice.")
        return self._values[idx][1]


    def __setitem__(self, p : Union[int, slice], v : T) -> None:
        if hasattr(v, "set_parent"):
            v.set_parent(self)
        if type(p) is int:
            self._setitem_single(p, v)
            self._merge_redundant()
            self._needs_update()
            return
        if type(p) is not slice:
            raise TypeError("Excepted int or slice!")
        start = p.start or 0
        stop = p.stop or INFINITY
        orig_value = self[stop]
        (start_idx, _) = self._setitem_single(start, v)
        (end_idx, hit_end) = self._find_index(stop)
        if not hit_end and stop < INFINITY:
            (end_idx, _) = self._setitem_single(stop, orig_value)
        if stop == INFINITY:
            end_idx += 1
        del self._values[start_idx + 1 : end_idx]
        self._merge_redundant()
        self._needs_update()
    
    def _setitem_single(self, p : int, v : T):
        (idx, hit) = self._find_index(p)
        if hit:
            self._values[idx] = (p, v)
        else:
            idx += 1
            self._values.insert(idx, (p, v))
        return (idx, hit)

    def _merge_redundant(self):
        for i in range(len(self._values) - 1, 0, -1):
            if self._values[i][1] == self._values[i-1][1]:
                del self._values[i]
    
    def __repr__(self) -> str:
        values = ", ".join([f"{page}: {value}" for (page, value) in self._values])
        return f"PageProperty{{{values}}}" 

    def __eq__(self, other):
        if type(other) != PageProperty:
            return False
        return self._values == other._values

    def map_values_in_place(self, f):
        for i in range(len(self._values)):
            (p, v) = self._values[i]
            self._values[i] = (p, f(v))

    def to_json(self) -> Dict[str, Any]:
        if len(self._values) == 1:
            return self._values[0][1]
        else:
            return {"type" : "PageProperty", "values" : self._values }
    
    @staticmethod
    def from_json(json_obj : Dict[str, Any]) -> "PageProperty[Any]":
        result : PageProperty[Any] = PageProperty(None)
        result._values = [cast(Tuple[int, Any], tuple(x)) for x in json_obj["values"]]
        return result

S = TypeVar('S')
PagePropertyOrValue = Union[S, PageProperty[S]]

def ensure_page_property(v : PagePropertyOrValue[S], parent : Optional[Any] = None) -> PageProperty[S]:
    if(type(v) is PageProperty):
        result = v
    else:
        result = PageProperty(v)
    if parent:
        result.set_parent(parent)
    return result