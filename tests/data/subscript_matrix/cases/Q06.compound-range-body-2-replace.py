def case():
    t = src()
    d = {}
    i = 0
    for i in range(1):
        d["a"] = t
        if True:
            if True:
                d = {}
            else:
                d = {}
        else:
            d = {}
    sink(d)
