def case():
    t = src()
    k = "n"
    d = {"u": {"n": "s0", "m": "s0"}}
    d["u"][k] = t # W
    sink(d["u"]["n"]) # R
