def case():
    t = src()
    global d
    d["a"] = t
    sink(d)

d = {}
