def case():
    t = src()
    d = {0: [None]}
    d[(d.clear(), 0)[1]] = t
    sink(d)
