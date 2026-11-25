def case():
    t = src()
    d = {"a": "", "b": ""}
    d["a"] = t
    sink((d))
