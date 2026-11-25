def case():
    t = src()
    d = {}; e = d
    d["a"] = t
    e["b"] = "clean"
    sink(d)
