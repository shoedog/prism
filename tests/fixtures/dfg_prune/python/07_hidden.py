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
