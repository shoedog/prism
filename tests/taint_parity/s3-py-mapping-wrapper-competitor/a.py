import os

def wrap(fn):
    def wrapper(**kw):
        return fn(**kw)
    return wrapper

@wrap
def run(*, x):
    os.system(x)  # @SINK

def entry():
    t = acquire()  # @SRC
    alias = run
    observe(alias(**{"x": t}), run(x="fixed"))
