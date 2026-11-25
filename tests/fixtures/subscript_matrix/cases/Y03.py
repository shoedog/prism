def case():
    t = src()
    k = "a"
    d = {"a": "s0", "b": "s0"}
    d["a"] = t # W
    sink(d[k]) # R
