def case():
    t = src()
    d = {}; e = d
    d["a"] = t
    e["a"] = "clean"
    sink(d)
