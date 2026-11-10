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


def h_eval_alias():
    ev = eval
    x = 1
    x = ev("x + 1")
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


import builtins
import builtins as bmod
from builtins import eval as ev_mod
ev_assign = eval
ev_chain = ev_assign


def h_builtins_attr():
    x = 1
    x = builtins.eval("x + 1")
    return x


def h_getattr():
    x = 1
    x = getattr(bmod, "eval")("x + 1")
    return x


def h_dunder():
    x = 1
    x = __builtins__["eval"]("x + 1")
    return x


def h_module_import_alias():
    x = 1
    x = ev_mod("x + 1")
    return x


def h_module_assign_alias():
    x = 1
    x = ev_chain("x + 1")
    return x


def h_string_only(ns):
    x = 1
    x = getattr(ns, "eval")("x + 1")
    return x
