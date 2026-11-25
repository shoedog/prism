def case():
    t = src()
    d = {"ab": "s0"}
    d["a" "b"] = t # W
    sink(d["ab"]) # R
