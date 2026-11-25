def case():
    t = src()
    d = {"a": "s0", "b": "s0"}
    d["a"] = t # W
    d["b"] = "safe"
    sink(d["b"]) # R
