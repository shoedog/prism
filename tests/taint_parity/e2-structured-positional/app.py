import pickle
def f(inp, suffix):
    t = inp.value # @SRC
    pickle.loads((t + suffix)) # @SINK
