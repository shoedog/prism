import os

def run(prefix, *args):
    os.system(args[0]) # @SINK

def main():
    t = acquire() # @SRC
    run('fixed', t)
main()
