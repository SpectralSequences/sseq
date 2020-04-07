
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

# Write x^n but handle special cases x^0 ==> 1 and x^1 ==> x
def power_name(var, n, zeroth_power=""):
    if n == 0:
        return zeroth_power
    elif n==1:
        return var
    else:
        return str(var) + "^{" + str(n) + "}"

def monomial_name(*exponents):
    result = ""
    for [var, e] in exponents:
        result += " " + power_name(var, e)
    if result.strip() == "":
        result = "1"
    return result
