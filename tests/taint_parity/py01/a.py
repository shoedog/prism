import os
def run(c):
    os.system(c)  # @SINK

def main():
    t = input()  # @SRC
    run(t)

main()
