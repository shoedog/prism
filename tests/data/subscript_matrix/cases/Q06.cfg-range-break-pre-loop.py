def case():
    t = src()
    d = {}
    i = 0
    for i in range(1):
        d["a"] = t
        break
    sink(d)
