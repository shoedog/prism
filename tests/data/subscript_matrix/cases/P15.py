def case():
    t = src()
    d = {"a": "s0"}
    d["a"] = t # W
    x = d["a"] # R
    sink(x)
