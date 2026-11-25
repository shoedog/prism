def case():
    t = src()
    d = {}
    d["a"] = t
    other = []
    other += [None]
    sink(d)
