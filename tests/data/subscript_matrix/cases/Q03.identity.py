def case():
    t = src()
    d = {}
    d["a"] = t
    x = id(d)
    sink(x)
