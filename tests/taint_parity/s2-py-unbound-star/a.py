import os
class Runner:
    def run(self, cmd):
        os.system(cmd) # @SINK

def entry():
    t = acquire() # @SRC
    Runner.run(Runner(), *(t,))

entry()
