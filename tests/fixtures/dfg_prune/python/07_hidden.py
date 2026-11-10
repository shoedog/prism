def h_eval():
    x = 1
    x = eval("x + 1")
    return x


def h_locals():
    x = 1
    x = locals()["x"] + 1
    return x


def h_format_locals():
    x = "a"
    x = "{x}b".format(**locals())
    return x


def c_member_eval(model):
    x = 1
    x = model.eval()
    return x


def c_keyword(f):
    x = 1
    x = f(eval=True)
    return x


def c_plain():
    x = 1
    x = 2
    return x
