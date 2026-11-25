def case():
    t = src()
    d = {}
    d["a"] = t
    x = "a" in d
    sink(x)
