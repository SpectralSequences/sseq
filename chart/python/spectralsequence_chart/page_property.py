from .infinity import INFINITY
import json

class PageProperty:
    def __init__(self, value):
        self.values = [[-INFINITY, value]]

    def find_index(self, target_page):
        for (idx, (page, v)) in enumerate(self.values):
            if page > target_page:
                break
            result_idx = idx 
        return [result_idx, self.values[result_idx][0] == target_page]

    def __getitem__(self, x):
        if type(x) == slice:
            raise TypeError("Can only assign to slice index, cannot retreive.")
        if type(x) != int:
            raise TypeError(f"Expected integer, got {type(x).__name__}.")
        [idx, _] = self.find_index(x)
        return self.values[idx][1]


    def __setitem__(self, p, v):
        if type(p) is int:
            self.setitem_single(p, v)
            self.merge_redundant()
            return
        if type(p) is not slice:
            raise TypeError("Excepted int or slice!")
        start = p.start or -INFINITY
        stop = p.stop or INFINITY
        orig_value = self[stop]
        [start_idx, hit_start] = self.setitem_single(start, v)
        [end_idx, hit_end] = self.find_index(stop)
        if not hit_end and stop < INFINITY:
            [end_idx, _] = self.setitem_single(stop, orig_value)
        if stop == INFINITY:
            end_idx += 1
        del self.values[start_idx + 1 : end_idx]
        self.merge_redundant()
    
    def setitem_single(self, p, v):
        [idx, hit] = self.find_index(p)
        if hit:
            self.values[idx][1] = v
        else:
            idx += 1
            self.values.insert(idx, [p, v])
        return [idx, hit]

    def merge_redundant(self):
        for i in range(len(self.values) - 1, 0, -1):
            if self.values[i][1] == self.values[i-1][1]:
                del self.values[i]
    
    def __repr__(self):
        return f"PageProperty({json.dumps(self.values)})"

    def to_json(self):
        return {"type" : "PageProperty", "data" : self.values }
    
    @staticmethod
    def from_json(json_obj):
        result = PageProperty(None)
        result.values = json_obj["data"]
        return result

        