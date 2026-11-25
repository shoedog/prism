def case():
    t = src()
    d = {"a": "s0"}
    d["a"], x = t, 1 # W
    sink(d["a"]) # R
