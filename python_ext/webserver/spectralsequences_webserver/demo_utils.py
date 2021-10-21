
def make_add_class_func(x_offset, y_offset, name="", **kwargs1):
    name_outer=name
    async def add_class(x, y, name="", **kwargs2):
        # print(name, name_outer)
        name = name + name_outer
        return await chart.add_class(x + x_offset, y+y_offset, name=name, **kwargs1, **kwargs2)
    return add_class

def make_add_structline_func(**kwargs1):
    async def add_structline(source, target, **kwargs2):
        return await chart.add_structline(source, target, **kwargs1, **kwargs2)
    return add_structline

