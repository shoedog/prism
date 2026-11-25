import os

def make():
    t = acquire() # @SRC
    return t + '!'

def main():
    out = make()
    os.system(out) # @SINK
main()
