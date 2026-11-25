def pair():
    t = acquire()  # @SRC
    return t, "fixed"

def entry():
    out = pair()
    observe(out)  # @SINK
