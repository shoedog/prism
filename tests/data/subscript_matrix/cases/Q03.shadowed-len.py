def case():
    t = src()
    d = {}
    d["a"] = t
    len = identity
    x = len(d)
    sink(x)

def identity(value):
    return value
