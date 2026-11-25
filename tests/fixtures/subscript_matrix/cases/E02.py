def case():
    t = src()
    class C:
        def __init__(self, v):
            self.n = v
    d = {"u": C("s0")}
    d["u"] = C(t) # W
    x = d["u"].n # R
    sink(x)
