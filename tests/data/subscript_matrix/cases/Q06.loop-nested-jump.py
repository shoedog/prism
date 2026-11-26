def case():
    t = src()
    d = {}
    for i in range(1):
        d["a"] = t
        for j in range(1):
            break
        continue
        d = {}
    sink(d)
