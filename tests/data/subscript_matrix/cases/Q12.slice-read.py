def case():
    t = src()
    d = [None]
    d[0] = t
    x = d[:1]
    sink(x)
