import rust_algebra

adem_algebras_dict = {}

def AdemAlgebra(p):
    if p not in adem_algebras_dict:
        adem_algebras_dict[p] = rust_algebra.algebra.AdemAlgebra(p)
    return adem_algebras_dict[p]
        
def MilnorAlgebra(p):
    if p not in adem_algebras_dict:
        adem_algebras_dict[p] = rust_algebra.algebra.MilnorAlgebra(p)
    return adem_algebras_dict[p]