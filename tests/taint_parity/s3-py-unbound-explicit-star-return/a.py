import os

class Worker:
    def echo(self, x):
        return x

def entry():
    t = acquire()  # @SRC
    y = Worker.echo(
        *(Worker(),),
        t
    )
    os.system(y)  # @SINK
