def case():
    d = {}
    i = 0
    while i < 1:
        i += 1
        t = src()
        d["a"] = t
        break
    else:
        spare = None
    sink(d)
