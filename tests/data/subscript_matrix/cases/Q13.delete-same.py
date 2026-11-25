def case():
    t = src()
    d = {"a": "", "b": ""}
    d["a"] = t
    del d["a"]
    sink(d)
