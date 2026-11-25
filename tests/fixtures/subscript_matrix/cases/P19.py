def case():
    t = src()
    d = {"a": t, "b": "safe"} # W
    sink(d["b"]) # R
