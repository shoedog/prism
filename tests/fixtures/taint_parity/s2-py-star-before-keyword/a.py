import os
def run(cmd, /, *, note):
    os.system(cmd) # @SINK
    observe(note)

def entry():
    t = acquire() # @SRC
    run(*('fixed',), note=t)

entry()
