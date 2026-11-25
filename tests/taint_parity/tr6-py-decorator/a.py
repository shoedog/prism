import os

def discard(fn):
    def wrapper(x):
        return fn('fixed')
    return wrapper
@discard
def run(x):
    os.system(x) # @SINK

def main():
    t = acquire() # @SRC
    run(t)
main()
