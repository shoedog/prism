import os
class Payload:
    pass

def run(x):
    os.system(x) # @SINK

def entry():
    t = acquire() # @SRC
    p = Payload()
    p.cmd = t
    run(p.cmd)

entry()
