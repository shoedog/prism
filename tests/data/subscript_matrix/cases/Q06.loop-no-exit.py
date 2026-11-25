def case():
    t = src()
    x = inner(t)
    sink(x)

def inner(t):
    d = {}
    while True:
        d["a"] = t
        return {}
    return d
