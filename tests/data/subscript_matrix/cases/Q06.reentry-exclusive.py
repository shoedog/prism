def case():
    t = src()
    d = {}
    i = 0
    while True:
        if i:
            x = d
            break
        else:
            d["a"] = t
        i += 1
    sink(x)
