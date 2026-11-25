def case():
    t = src()
    d = {"a": "s0"}
    for d["a"] in [t]: # W
        pass
    x = d["a"] # R
    sink(x)
