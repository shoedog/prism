import os

def inner(v):
    return v

def forget(v):
    observe(v)
    return "fixed"

def entry():
    t = acquire()  # @SRC
    out = forget(inner(t))
    os.system(out)  # @SINK
