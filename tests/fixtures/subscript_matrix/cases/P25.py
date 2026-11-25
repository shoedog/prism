def case():
    t = src()
    class C:
        def __init__(self):
            self.n = "s0"
            self.m = "s0"
            self.c = "s0"
    d = {"a": "s0", "b": "s0"}
    d["a"] = C()
    d["b"] = C()
    d["a"].c = t # W
    d["b"].c = "x"
    sink(d["a"].c) # R
