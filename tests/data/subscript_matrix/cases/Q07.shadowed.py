def case():
    t = src()
    d = {}
    d["a"] = t
    def inner():
        d = {}
        return d
    x = inner()
    sink(x)
