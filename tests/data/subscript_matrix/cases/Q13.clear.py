def case():
    t = src()
    d = {"a": "", "b": ""}
    d["a"] = t
    d.clear()
    sink(d)
