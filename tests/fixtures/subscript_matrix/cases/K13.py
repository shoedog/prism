def case():
    t = src()
    d = {"a": "s0"}
    d[r"a"] = t # W
    sink(d["a"]) # R
