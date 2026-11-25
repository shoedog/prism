def case():
    t = src()
    k = "b"
    d = {"a": "s0", "b": "s0"}
    d[k] = t # W
    sink(d["a"]) # R
