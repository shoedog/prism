import os
def f(x):
    return x

def main():
    t = input()  # @SRC
    y = f(t)
    os.system(y)  # @SINK

main()
