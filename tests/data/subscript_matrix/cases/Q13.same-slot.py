def case():
    t = src()
    d = {"a": "", "b": ""}
    d["a"] = t
    d["a"] = "clean"
    sink(d)
