def case():
    t = src()
    d = {"a": "s0", "b": "s0"}
    d["a"] = t # W
    d["b"] = "safe"
    x = d["b"] # R
    sink(x)
