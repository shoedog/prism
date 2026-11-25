def case():
    t = src()
    d = {"a": "", "b": ""}
    d["a"], other = t, 0
    sink(d)
