def case():
    t = src()
    d = {}
    if True:
        d["a"] = t
        if True:
            spare = 1
        else:
            d = {}
    else:
        d = {}
    sink(d)
