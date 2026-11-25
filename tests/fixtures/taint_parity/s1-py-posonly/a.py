import os

def run(x, /):
    os.system(x) # @SINK

def main():
    t = acquire() # @SRC
    run(t)
main()
