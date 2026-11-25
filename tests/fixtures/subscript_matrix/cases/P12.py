def case():
    t = src()
    k = "q"
    d = {"q": "s0", "b": "s0"}
    d[k] = t # W
    d["b"] = "safe"
    sink(d["b"]) # R
