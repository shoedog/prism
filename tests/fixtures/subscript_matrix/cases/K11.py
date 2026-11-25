def case():
    t = src()
    p = "a"
    d = {"a": "s0"}
    d[f"{p}"] = t # W
    sink(d["a"]) # R
