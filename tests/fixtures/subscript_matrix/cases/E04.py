def case():
    t = src()
    d = {"u": {"n": "s0"}}
    d["u"] = {"n": t} # W
    sink(d["u"].get("n")) # R
