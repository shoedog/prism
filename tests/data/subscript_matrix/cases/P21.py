def case():
    t = src()
    d = {"a": "s0"}
    d["a"] = t # W
    d["a"] += "x"
    sink(d) # R
