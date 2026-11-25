def case():
    t = src()
    xs = ["s0", "s0"]
    xs[0] = t # W
    sink(xs[0]) # R
