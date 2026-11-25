def case():
    t = src()
    d = {"k": "s0"}
    d["k"] = t # W
    del d["k"]
    d["k"] = "c"
    sink(d["k"]) # R
