def case():
    t = src()
    d = [None]
    d[:1] = [t]
    sink(d)
