def g(p):
    sink(p)  # @SINK

def f():
    user = input()  # @SRC
    g(user); g(user)
