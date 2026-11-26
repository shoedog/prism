def case():
    t = src()
    d = {}
    d["a"] = t
    x = len(d)
    sink(x)
