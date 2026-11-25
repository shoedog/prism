def case():
    t = src()
    d = [[None]]
    d[0][0] = t
    d[0] = [None]
    sink(d)
