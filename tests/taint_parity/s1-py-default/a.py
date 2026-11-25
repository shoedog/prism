import os

def run(x='fixed'):
    os.system(x) # @SINK

def main():
    t = acquire() # @SRC
    run(t)
main()
