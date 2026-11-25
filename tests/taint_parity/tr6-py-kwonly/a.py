import os

def run(*, cmd):
    os.system(cmd) # @SINK

def main():
    t = acquire() # @SRC
    run(cmd=t)
main()
