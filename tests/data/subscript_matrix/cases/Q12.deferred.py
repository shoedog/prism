def case():
    t = src()
    d = {"a": "", "b": ""}
    d["a"] = t
    f = lambda: d
    x = f()
    sink(x)
