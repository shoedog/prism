def case():
    t = src()
    d = {}
    d["a"] = t
    if False:
        d = {}
    elif True:
        spare = 0
    else:
        d = {}
    sink(d)
