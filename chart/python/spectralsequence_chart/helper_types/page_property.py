from ..infinity import INFINITY
import json
from typing import List, Tuple, Any, Union, TypeVar, Generic, Optional, Dict

T = TypeVar('T')
class PageProperty(Generic[T]):
    def __init__(self, value : T, parent : Optional[Any] = None):
        self.values : List[Tuple[int, T]] = [(-INFINITY, value)]
        self.set_parent(parent)

    def set_parent(self, parent : Optional[Any]):
        self._parent = parent
    
    def needs_update(self):
        if self._parent:
            self._parent.needs_update()

    def find_index(self, target_page : int) -> Tuple[int, bool]:
        result_idx = None
        for (idx, (page, _)) in enumerate(self.values):
            if page > target_page:
                break
            result_idx = idx 
        if result_idx is None:
            assert False, "Unreachable"
        return (result_idx, self.values[result_idx][0] == target_page)

    def __getitem__(self, x : Union[int, slice]) -> T:
        if type(x) == slice:
            raise TypeError("Can only assign to slice index, cannot retreive.")
        if type(x) != int:
            raise TypeError(f"Expected integer, got {type(x).__name__}.")
        assert type(x) is int # Make type analysis thing happy
        (idx, _) = self.find_index(x)
        return self.values[idx][1]


    def __setitem__(self, p : Union[int, slice], v : T) -> None:
        if type(p) is int:
            self.setitem_single(p, v)
            self.merge_redundant()
            return
        if type(p) is not slice:
            raise TypeError("Excepted int or slice!")
        start = p.start or -INFINITY
        stop = p.stop or INFINITY
        orig_value = self[stop]
        (start_idx, _) = self.setitem_single(start, v)
        (end_idx, hit_end) = self.find_index(stop)
        if not hit_end and stop < INFINITY:
            (end_idx, _) = self.setitem_single(stop, orig_value)
        if stop == INFINITY:
            end_idx += 1
        del self.values[start_idx + 1 : end_idx]
        self.merge_redundant()
        self.needs_update()
    
    def setitem_single(self, p : int, v : T):
        (idx, hit) = self.find_index(p)
        if hit:
            self.values[idx] = (p, v)
        else:
            idx += 1
            self.values.insert(idx, (p, v))
        return (idx, hit)

    def merge_redundant(self):
        for i in range(len(self.values) - 1, 0, -1):
            if self.values[i][1] == self.values[i-1][1]:
                del self.values[i]
    
    def __repr__(self) -> str:
        return f"PageProperty({json.dumps(self.values)})"

    def to_json(self) -> Dict[str, Any]:
        return {"type" : "PageProperty", "data" : self.values }
    
    @staticmethod
    def from_json(json_obj : Dict[str, Any]) -> "PageProperty[Any]":
        result = PageProperty(None)
        result.values = json_obj["data"]
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