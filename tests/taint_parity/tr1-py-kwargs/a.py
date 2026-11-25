import os

def run(**kwargs):
    os.system(kwargs['cmd']) # @SINK

def main():
    t = acquire() # @SRC
    run(cmd=t)

main()
