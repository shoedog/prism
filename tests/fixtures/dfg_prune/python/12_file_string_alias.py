def make(ns):
    return getattr(ns, "eval")


def f(run):
    x = 1
    x = run("x + 1")
    return x
