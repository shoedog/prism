def case():
    t = src()
    x = make(t)
    sink(x)

def make(value):
    d = {}
    d["a"] = value
    return d
