def format_power(var, n, zeroth_power=""):
    if n == 0:
        return zeroth_power
    elif n==1:
        return var
    else:
        # if var.find("'") > -1:
        #     var = f"({var})"
        return f"{var}^{{{n}}}"

def format_monomial(**exponents):
    result = " ".join(format_power(var, e) for [var,e] in exponents if e)
    if result.strip() == "":
        result = "1"
    return result