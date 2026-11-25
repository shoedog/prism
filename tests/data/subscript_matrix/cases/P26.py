def case():
    t = src()
    class C:
        def __init__(self):
            self.n = "s0"
            self.m = "s0"
            self.c = "s0"
    d = {"a": "s0"}
    d["a"] = C()
    d["a"].c = t # W
    d["a"].c = "clean"
    sink(d["a"].c) # R
