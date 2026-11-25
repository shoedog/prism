import os

def main():
    t = acquire() # @SRC
    def run():
        os.system(t) # @SINK
    run()
main()
