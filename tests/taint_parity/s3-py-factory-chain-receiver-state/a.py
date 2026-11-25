import os

class Store:
    def __init__(self, x):
        self.x = x

    def read(self):
        os.system(self.x)  # @SINK

def factory(x):
    return Store(x)

def entry():
    t = acquire()  # @SRC
    return factory(t).read()
