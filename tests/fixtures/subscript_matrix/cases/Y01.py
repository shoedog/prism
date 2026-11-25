def case():
    t = src()
    k = "a"
    d = {"a": "s0", "b": "s0"}
    d[k] = t # W
    sink(d["a"]) # R
