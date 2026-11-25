def case():
    t = src()
    d = {"a": "", "b": ""}
    d["a"] = t
    copy = d
    x = d["a"]
    sink(x)
