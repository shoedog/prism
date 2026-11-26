def case():
    t = src()
    d = {"a": "", "b": ""}
    d["a"] = t
    e = d
    sink(e)
