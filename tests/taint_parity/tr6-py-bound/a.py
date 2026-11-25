import os
class Runner:
    def run(self, cmd):
        os.system(cmd) # @SINK

def main():
    t = acquire() # @SRC
    Runner().run(t)
main()
