def case():
    t = src()
    d = [None, None, None]
    d[::2] = [t, None]
    sink(d)
