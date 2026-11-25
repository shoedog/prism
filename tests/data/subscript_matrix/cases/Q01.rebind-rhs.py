def case():
    t = src()
    d = {"a": "", "b": ""}
    d["a"] = ((d := {}) or t)
    sink(d)
