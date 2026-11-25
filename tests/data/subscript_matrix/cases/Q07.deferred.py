def case():
    t = src()
    d = {}
    d["a"] = t
    x = (d for _ in [0])
    y = next(x)
    sink(y)
