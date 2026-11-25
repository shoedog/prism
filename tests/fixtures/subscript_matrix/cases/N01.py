def case():
    t = src()
    d = {"a": "s0", "b": "s0"}
    d["a"] = t # W
    e = d
    e["a"] = "safe"
    sink(d["a"]) # R
