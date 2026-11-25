import os

def run(a, b):
    os.system(b) # @SINK

def main():
    t = acquire() # @SRC
    run(*[], t)

main()
