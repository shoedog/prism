import os

class Runner:
    @classmethod
    def run(cls, x):
        os.system(x) # @SINK

def main():
    t = acquire() # @SRC
    Runner.run(t)
main()
