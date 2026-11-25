def case():
    t = src()
    d = {"u": {"n": "s0", "m": "s0"}}
    d["u"]["n"] = t # W
    d["u"]["m"] = "safe"
    sink(d["u"]["m"]) # R
