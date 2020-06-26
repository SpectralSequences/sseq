class PageProperty:
    def __init__(self, page_list):
        self.page_list = page_list
        self.property_list = [None] * (len(page_list) + 1)

    def find_index_get(self, target_p):
        for (idx, cur_p) in self.page_list:
            if cur_p > target_p:
                return idx

    def find_index_set(self, target_p):
        try:
            return self.page_list.index(p.target_p)
        except ValueError:
            raise IndexError(f"{p.start} is not a page transition.")

    def __getitem__(self, x):
        if type(x) == slice:
            raise TypeError("Can only assign to slice index, cannot retreive.")
        if type(x) != int:
            raise TypeError(f"Expected integer, got {type(x).__name__}.")
        idx = self.find_index_get(target_p)
        return self.property_list[idx]


    def __setitem__(self, p, v):
        if type(p) == slice:
            min_idx = self.find_index_set(p.start)
            max_idx = self.find_index_set(p.stop)
        else:
            min_idx = self.find_index_set(p)
        
        for i in range(min_idx, max_idx + 1):
            self.property_list[i] = v

    def to_json(self):
        result = [None] * len(self.property_list)
        last_value = None
        for (i, v) in enumerate(self.property_list):
            if v != last_value:
                result[i] = v
            last_value = v
                
        return {"type" : "PageProperty", "data" : self.property_list }
    
    def __repr__(self):
        result = {}
        last_value = self.property_list[0]
        for (i, v) in enumerate(self.property_list):
            result["0--infty"] = last_value
            if v != last_value:
                result[i] = v
            last_value = v
            
        return "PageProperty()"

    @staticmethod
    def from_json(page_list, json_obj):
        result = PageProperty(page_list)
        json_props = json_obj["data"]
        if len(json_props) != len(page_list) + 1:
            raise ValueError(
                f"Property list should be one longer than page list, \
                but property list has length {len(json_props)} and page list has length {len(page_list)}")
        last_value = None
        for (i, v) in enumerate(json_props):
            result.property_list[i] = v if v is not None else last_value

        