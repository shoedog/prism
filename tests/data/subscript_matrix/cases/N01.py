def case():
    t = src()
    d = {"a": "s0", "b": "s0"}
    d["a"] = t # W
    e = d # R1
    e["a"] = "safe"
    sink(d) # R
