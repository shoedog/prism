def case():
    t = src()
    d = {"0": "s0"}
    d[0] = t # W
    sink(d["0"]) # R
