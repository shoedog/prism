def case():
    t = src()
    d = {"a": "s0", "b": "s0"}
    d["a"] = t # W
    d = {"a": "s0"}
    sink(d["a"]) # R
