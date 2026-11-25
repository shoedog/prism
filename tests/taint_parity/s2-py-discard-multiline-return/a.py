import os
def identity(x):
    return x

def entry():
    t = acquire() # @SRC
    identity(
        t); fixed = identity('fixed')
    os.system(fixed) # @SINK

entry()
