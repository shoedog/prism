def case():
    t = src()
    d = {
        "a": t, # W
        "b": "s",
    }
    sink(d["a"]) # R
