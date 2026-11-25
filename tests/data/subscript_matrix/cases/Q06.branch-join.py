def case():
    t = src()
    d = {}
    if True:
        d["a"] = t
    else:
        d = {}
    sink(d)
