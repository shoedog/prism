import os

def outer():
    def child():
        t = acquire()  # @SRC
        return t
    child()
    return "fixed"

def entry():
    out = outer()
    os.system(out)  # @SINK
