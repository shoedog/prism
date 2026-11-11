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
