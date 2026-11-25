import os
def f(a, b):
    os.system(a)
    print(b) # @SINK
def main():
    t = get_input()  # @SRC
    f(b=t, a='fixed')
main()
