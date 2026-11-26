def case():
    t = src()
    d = {}
    i = 0
    while i < 1:
        i += 1
        d["a"] = t
        if True:
            d = {}
        else:
            d = {}
    sink(d)
