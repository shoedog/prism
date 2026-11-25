def case():
    t = src()
    k = "x"
    d = {"x": {"a": "s0"}}
    d[k]["a"] = t # W
    sink(d["x"]["a"]) # R
