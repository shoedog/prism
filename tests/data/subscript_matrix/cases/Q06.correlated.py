def case():
    t = src()
    d = {}; flag = True
    if flag:
        d["a"] = t
    if flag:
        d = {}
    sink(d)
