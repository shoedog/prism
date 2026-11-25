def case():
    t = src()
    d = {}
    d["a"] = t
    noop()
    noop()
    noop()
    sink(d)

def noop():
    pass
