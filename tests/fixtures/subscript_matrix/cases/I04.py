def case():
    t = src()
    d = {
        "a": ( # W
            t
        ),
        "b": "s",
    }
    sink(d["a"]) # R
