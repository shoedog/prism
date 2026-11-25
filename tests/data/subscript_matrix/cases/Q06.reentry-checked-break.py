def case():
    t = src()
    d = {}
    i = 0
    while True:
        if i:
            break
        d["a"] = t
        i += 1
    sink(d)
