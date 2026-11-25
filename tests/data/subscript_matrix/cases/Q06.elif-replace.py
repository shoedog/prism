def case():
    t = src()
    d = {}
    d["a"] = t
    if False:
        d = {}
    elif True:
        d = {}
    else:
        d = {}
    sink(d)
