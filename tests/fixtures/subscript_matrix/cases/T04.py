def case():
    t = src()
    d = {"a": "s0"}
    for d["a"] in [t]: # W
        pass
    sink(d["a"]) # R
