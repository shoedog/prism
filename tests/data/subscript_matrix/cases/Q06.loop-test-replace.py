def case():
    t = src()
    d = {}
    d["a"] = t
    while (d := {}):
        pass
    sink(d)
