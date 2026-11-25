def case():
    t = src()
    d = {0: [None]}
    d[0][(d.clear(), 0)[1]] = t
    sink(d)
