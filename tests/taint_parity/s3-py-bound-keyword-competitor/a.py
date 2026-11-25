import os

class Worker:
    def run(self, *, x):
        os.system(x)  # @SINK

def entry(other):
    t = acquire()  # @SRC
    observe(other.run(x=t), Worker.run(Worker(), x="fixed"))

class Other:
    def run(self, *, x):
        observe(x)

entry(Other())
