def case():
    t = src()
    class C:
        def __init__(self, v):
            self.n = v
    d = {"u": C("s0")}
    d["u"] = C(t) # W
    f(d["u"].n) # R

def f(p):
    sink(p)
