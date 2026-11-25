def case():
    t = src()
    d = {}
    d["a"] = t
    def inner():
        return d
    x = inner()
    sink(x)
