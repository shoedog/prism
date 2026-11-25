import os
def run(cmd, /, **extras):
    os.system(cmd) # @SINK
    observe(extras)

def entry():
    t = acquire() # @SRC
    run('fixed', **{'cmd': t})

entry()
