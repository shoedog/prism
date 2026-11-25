def case():
    t = src()
    d = {"a": "s0"}
    d["a"] = t # W
    d = {"a": "s0"}
    sink(d) # R
