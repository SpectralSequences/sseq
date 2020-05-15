from lark import Lark
_name_parser = Lark("""
    ?start : name
    name : var*

    var : gen _extras

    _extras : (sub _extras_sub) | (sup _extras_sup) | (prime _extras) | ()
    _extras_sub : (sup [prime]) | (prime _extras_sub) | ()
    _extras_sup : (sub [prime]) | (prime _extras_sup) | ()

    prime : "'"
    sub : "_" arg

    sup : "^" arg

    gen : /[A-Za-z]/

    ?arg : ("{" /\w+/ "}") | /[0-9]+/

    %ignore " "
""")

from lark import Tree, Transformer
class EvalName(Transformer):
    def gen(self, args):
       return str(args[0])

    def sub(self, args):
        return ["add_to_var", f"_{{{args[0]}}}"]

    def sup(self,args):
        return ["sup", str(args[0])]

    def var(self, args):
        name = args[0]
        result = []
        for arg in sorted(args[1:]):
           if arg[0] == "add_to_var":
              name += arg[1]
           else:
              result.append(int(arg[1]))
        result = [name] + result
        if len(result) == 1:
            result.append(1)
        return result

    def prime(self, args):
        return ["add_to_var", "'"]

    def name(self, args):
        return args

_name_evaluator = EvalName()

def parse_name(name):
    t = _name_parser.parse(name)
    return _name_evaluator.transform(t)

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
