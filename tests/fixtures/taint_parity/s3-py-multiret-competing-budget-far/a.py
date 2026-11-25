import os

def relay(x):
    return x, "fixed"

def near(x):
    far(x)

def far(x):
    os.system(x)  # @SINK

def entry():
    t = acquire()  # @SRC
    x, _ = relay(
        t
    )
    near(x)
    near(t)
