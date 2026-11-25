def case():
    t = src()
    d = {"a": "", "b": ""}
    d["a"] = t
    del d["b"]
    sink(d)
