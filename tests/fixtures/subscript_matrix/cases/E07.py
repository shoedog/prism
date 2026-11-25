def case():
    t = src()
    class C:
        def __init__(self, v):
            self.n = v
    d = {"u": C("s0"), "w": C("s0")}
    d["u"] = C(t) # W
    sink(d["w"].n) # R
