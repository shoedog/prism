def case():
    t = src()
    k = "b"
    d = {"a": "s0", "b": "s0"}
    d["a"] = t # W
    sink(d[k]) # R
