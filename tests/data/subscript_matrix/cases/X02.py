def case():
    t = src()
    d = {"k": "s0"}
    d["k"] = t # W
    del d["k"]
    sink(d) # R
