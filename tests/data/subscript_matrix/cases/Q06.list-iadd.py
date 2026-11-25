def case():
    t = src()
    d = [None]
    d[0] = t
    d += [None]
    sink(d)
