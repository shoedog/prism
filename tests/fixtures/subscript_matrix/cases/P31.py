def case():
    t = src()
    d = {"a": "s0"}
    d["a"] = ""
    d["a"] += t # W
    sink(d["a"]) # R
