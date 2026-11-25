def case():
    t = src()
    d = {"a": "", "b": ""}
    d["a"] = t
    d["b"] = "clean"
    sink(d)
