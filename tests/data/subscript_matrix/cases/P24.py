def case():
    t = src()
    class C:
        def __init__(self):
            self.n = "s0"
            self.m = "s0"
            self.c = "s0"
    d = {"u": "s0"}
    d["u"] = C()
    d["u"].n = t # W
    sink(d["u"].n) # R
