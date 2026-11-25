import os
def leaf(x):
    os.system(x) # @SINK

class Runner:
    def run(self, x):
        leaf(x)

def entry():
    t = acquire() # @SRC
    Runner().run(t)

entry()
