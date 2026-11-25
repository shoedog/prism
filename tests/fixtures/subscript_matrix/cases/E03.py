def case():
    t = src()
    sink(h(t))

def h(t):
    class C:
        def __init__(self, v):
            self.n = v
    d = {"u": C("s0")}
    d["u"] = C(t) # W
    return d["u"].n # R
