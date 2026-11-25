import os
class Runner:
    def run(self, x):
        os.system(x) # @SINK

def main():
    t = acquire() # @SRC
    Runner.run(Runner(), t)
main()
