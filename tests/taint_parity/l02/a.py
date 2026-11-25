import os
def g(c):
    os.system(c)  # @SINK

def main():
    t = input()  # @SRC
    u = 'fixed'
    g(t); g(u)
