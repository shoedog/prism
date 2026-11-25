def case():
    t = src()
    d = {"a": "s0", "b": "s0"}
    d["\x61"] = t # W
    sink(d["a"]) # R
