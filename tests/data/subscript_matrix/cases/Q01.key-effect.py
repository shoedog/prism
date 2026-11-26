def case():
    t = src()
    d = {"a": "", "b": ""}
    d[(key := "a")] = t
    sink(d)
