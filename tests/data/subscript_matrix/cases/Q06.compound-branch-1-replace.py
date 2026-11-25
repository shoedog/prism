def case():
    t = src()
    d = {}
    if True:
        d["a"] = t
        if True:
            d = {}
        else:
            d = {}
    else:
        d = {}
    sink(d)
