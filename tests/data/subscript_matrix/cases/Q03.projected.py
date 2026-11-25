def case():
    t = src()
    d = {}
    d["a"] = t
    x = d["a"]
    sink(x)
